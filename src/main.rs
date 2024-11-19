use std::collections::HashMap;
use rand::{
    rngs::StdRng, 
    RngCore, 
    SeedableRng,
    rngs::OsRng
};
use sha2::{Sha256, Digest};
use ed25519_dalek::{
    Keypair, 
    PublicKey, 
    Signature, 
    Signer, 
    Verifier, 
};
use std::hash::{Hash, Hasher};

#[derive(Clone, Debug)]
struct PublicKeyWrapper(PublicKey);

impl Hash for PublicKeyWrapper {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_bytes().hash(state);
    }
}

impl PartialEq for PublicKeyWrapper {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bytes() == other.0.to_bytes()
    }
}

impl Eq for PublicKeyWrapper {}

impl From<PublicKey> for PublicKeyWrapper {
    fn from(pk: PublicKey) -> Self {
        PublicKeyWrapper(pk)
    }
}

impl AsRef<PublicKey> for PublicKeyWrapper {
    fn as_ref(&self) -> &PublicKey {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ConsensusPhase {
    Initialization,
    Preparation,
    PreCommitment,
    Commitment,
    Execution,
    Failed,
}

#[derive(Clone, Debug)]
struct Validator {
    id: PublicKeyWrapper,
    stake: u64,
    performance_score: f64,
    keypair: Vec<u8>, // Serialized keypair
}

#[derive(Clone, Debug)]
struct PreparationMessage {
    validator_id: PublicKeyWrapper,
    round_id: Vec<u8>,
    payload_hash: Vec<u8>,
    timestamp: u64,
    signature: Signature,
}

#[derive(Clone, Debug)]
struct PreCommitmentMessage {
    validator_id: PublicKeyWrapper,
    round_id: Vec<u8>,
    preparation_hashes: Vec<Vec<u8>>,
    proposed_execution_block: u64,
    signature: Signature,
}

#[derive(Clone, Debug)]
struct CommitmentMessage {
    validator_id: PublicKeyWrapper,
    round_id: Vec<u8>,
    final_payload_hash: Vec<u8>,
    execution_block: u64,
    signature: Signature,
}

struct MsgConsensus {
    validators: Vec<Validator>,
    committee: Vec<PublicKeyWrapper>,
    current_phase: ConsensusPhase,
    round_id: Vec<u8>,
    preparations: HashMap<PublicKeyWrapper, PreparationMessage>,
    pre_commitments: HashMap<PublicKeyWrapper, PreCommitmentMessage>,
    commitments: HashMap<PublicKeyWrapper, CommitmentMessage>,
    byzantine_tolerance: usize,
}

impl MsgConsensus {
    fn new(total_validators: usize) -> Self {
        let mut rng = StdRng::from_entropy();
        let validators: Vec<Validator> = (0..total_validators)
            .map(|_| {
                let keypair = Keypair::generate(&mut OsRng);
                Validator {
                    id: PublicKeyWrapper(keypair.public),
                    stake: rng.next_u64() % 90000 + 10000,
                    performance_score: (rng.next_u32() as f64 / u32::MAX as f64) * 0.5 + 0.5,
                    keypair: keypair.to_bytes().to_vec(),
                }
            })
            .collect();

        let byzantine_tolerance = total_validators / 3;
        let committee = Self::select_committee(&validators, byzantine_tolerance * 3 + 1);

        MsgConsensus {
            validators,
            committee,
            current_phase: ConsensusPhase::Initialization,
            round_id: Self::generate_round_id(),
            preparations: HashMap::new(),
            pre_commitments: HashMap::new(),
            commitments: HashMap::new(),
            byzantine_tolerance,
        }
    }

    fn generate_round_id() -> Vec<u8> {
        let mut rng = StdRng::from_entropy();
        let mut round_id = [0u8; 32];
        rng.fill_bytes(&mut round_id);
        round_id.to_vec()
    }

    fn select_committee(validators: &[Validator], committee_size: usize) -> Vec<PublicKeyWrapper> {
        let mut rng = StdRng::from_entropy();
        let mut sorted_validators = validators.to_vec();
        
        sorted_validators.sort_by(|a, b| 
            (b.stake as f64 * b.performance_score)
            .partial_cmp(&(a.stake as f64 * a.performance_score))
            .unwrap()
        );

        sorted_validators
            .into_iter()
            .take(committee_size)
            .map(|v| v.id)
            .collect()
    }

    fn prepare(&mut self, payload: Vec<u8>) -> Result<(), String> {
        if self.current_phase != ConsensusPhase::Initialization {
            return Err("Invalid phase for preparation".to_string());
        }

        let payload_hash = Self::hash_payload(&payload);

        for validator_id in &self.committee {
            let validator = self.validators.iter()
                .find(|v| v.id == *validator_id)
                .ok_or("Validator not found")?;

            let preparation = self.create_preparation_message(validator, &payload_hash)?;
            self.preparations.insert(validator.id.clone(), preparation);
        }

        if self.validate_preparations() {
            self.current_phase = ConsensusPhase::Preparation;
            Ok(())
        } else {
            self.current_phase = ConsensusPhase::Failed;
            Err("Preparation validation failed".to_string())
        }
    }

    fn create_preparation_message(
        &self, 
        validator: &Validator, 
        payload_hash: &[u8]
    ) -> Result<PreparationMessage, String> {
        let keypair_bytes = validator.keypair.clone();
        let keypair = Keypair::from_bytes(&keypair_bytes)
            .map_err(|_| "Failed to reconstruct keypair".to_string())?;

        let message = PreparationMessage {
            validator_id: validator.id.clone(),
            round_id: self.round_id.clone(),
            payload_hash: payload_hash.to_vec(),
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
            signature: keypair.sign(&[
                self.round_id.as_slice(), 
                payload_hash
            ].concat()),
        };
        Ok(message)
    }

    fn validate_preparations(&self) -> bool {
        let valid_preparations = self.preparations.values()
            .filter(|prep| self.verify_preparation(prep))
            .count();

        valid_preparations >= 2 * self.byzantine_tolerance + 1
    }

    fn verify_preparation(&self, prep: &PreparationMessage) -> bool {
        self.committee.contains(&prep.validator_id) &&
        prep.validator_id.0.verify(
            &[
                self.round_id.as_slice(), 
                prep.payload_hash.as_slice()
            ].concat(), 
            &prep.signature
        ).is_ok()
    }

    fn hash_payload(payload: &[u8]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(payload);
        hasher.finalize().to_vec()
    }

    fn pre_commit(&mut self) -> Result<(), String> {
        if self.current_phase != ConsensusPhase::Preparation {
            return Err("Invalid phase for pre-commitment".to_string());
        }

        let preparation_hashes: Vec<Vec<u8>> = self.preparations
            .values()
            .map(|prep| prep.payload_hash.clone())
            .collect();

        for validator_id in &self.committee {
            let validator = self.validators.iter()
                .find(|v| v.id == *validator_id)
                .ok_or("Validator not found")?;

            let keypair_bytes = validator.keypair.clone();
            let keypair = Keypair::from_bytes(&keypair_bytes)
                .map_err(|_| "Failed to reconstruct keypair".to_string())?;

            let pre_commitment = PreCommitmentMessage {
                validator_id: validator.id.clone(),
                round_id: self.round_id.clone(),
                preparation_hashes: preparation_hashes.clone(),
                proposed_execution_block: self.calculate_execution_block(),
                signature: keypair.sign(&[
                    self.round_id.as_slice(),
                    preparation_hashes.concat().as_slice()
                ].concat()),
            };

            self.pre_commitments.insert(validator.id.clone(), pre_commitment);
        }

        if self.validate_pre_commitments() {
            self.current_phase = ConsensusPhase::PreCommitment;
            Ok(())
        } else {
            self.current_phase = ConsensusPhase::Failed;
            Err("Pre-commitment validation failed".to_string())
        }
    }

    fn validate_pre_commitments(&self) -> bool {
        let valid_pre_commitments = self.pre_commitments.values()
            .filter(|pre_commit| self.verify_pre_commitment(pre_commit))
            .count();

        valid_pre_commitments >= 2 * self.byzantine_tolerance + 1
    }

    fn verify_pre_commitment(&self, pre_commit: &PreCommitmentMessage) -> bool {
        self.committee.contains(&pre_commit.validator_id) &&
        pre_commit.validator_id.0.verify(
            &[
                self.round_id.as_slice(), 
                pre_commit.preparation_hashes.concat().as_slice()
            ].concat(), 
            &pre_commit.signature
        ).is_ok()
    }

    fn commit(&mut self) -> Result<(), String> {
        if self.current_phase != ConsensusPhase::PreCommitment {
            return Err("Invalid phase for commitment".to_string());
        }

        for validator_id in &self.committee {
            let validator = self.validators.iter()
                .find(|v| v.id == *validator_id)
                .ok_or("Validator not found")?;

            let keypair_bytes = validator.keypair.clone();
            let keypair = Keypair::from_bytes(&keypair_bytes)
                .map_err(|_| "Failed to reconstruct keypair".to_string())?;

            let execution_block = self.pre_commitments
                .get(&validator.id)
                .map(|pc| pc.proposed_execution_block)
                .ok_or("Execution block not found")?;

            let final_payload_hash = self.get_final_payload_hash();

            // Create signature for the commitment
            let commitment_signature = keypair.sign(&[
                self.round_id.as_slice(), 
                &final_payload_hash,
                &execution_block.to_le_bytes()
            ].concat());

            let commitment = CommitmentMessage {
                validator_id: validator.id.clone(),
                round_id: self.round_id.clone(),
                final_payload_hash: final_payload_hash.clone(),
                execution_block,
                signature: commitment_signature,
            };

            self.commitments.insert(validator.id.clone(), commitment);
        }

        if self.validate_commitments() {
            self.current_phase = ConsensusPhase::Commitment;
            Ok(())
        } else {
            self.current_phase = ConsensusPhase::Failed;
            Err("Commitment validation failed".to_string())
        }
    }

    fn validate_commitments(&self) -> bool {
        let valid_commitments = self.commitments.values()
            .filter(|commit| self.verify_commitment(commit))
            .count();

        valid_commitments >= 2 * self.byzantine_tolerance + 1
    }

    fn verify_commitment(&self, commit: &CommitmentMessage) -> bool {
        if !self.committee.contains(&commit.validator_id) {
            return false;
        }

        commit.validator_id.0.verify(
            &[
                self.round_id.as_slice(), 
                &commit.final_payload_hash,
                &commit.execution_block.to_le_bytes()
            ].concat(), 
            &commit.signature
        ).is_ok()
    }

    fn execute(&mut self) -> Result<(), String> {
        if self.current_phase != ConsensusPhase::Commitment {
            return Err("Invalid phase for execution".to_string());
        }

        if self.validate_final_state() {
            self.current_phase = ConsensusPhase::Execution;
            println!("Cross-chain message execution confirmed!");
            Ok(())
        } else {
            self.current_phase = ConsensusPhase::Failed;
            Err("Final state validation failed".to_string())
        }
    }

    fn validate_final_state(&self) -> bool {
        let valid_commitments = self.commitments.values()
            .filter(|commit| self.verify_commitment(commit))
            .count();

        valid_commitments >= 2 * self.byzantine_tolerance + 1
    }

    fn calculate_execution_block(&self) -> u64 {
        let mut rng = StdRng::from_entropy();
        rng.next_u64() % 1000 + 1000
    }

    fn get_final_payload_hash(&self) -> Vec<u8> {
        let mut hasher = Sha256::new();
        let mut payload_bytes = Vec::new();
        
        for prep in self.preparations.values() {
            payload_bytes.extend_from_slice(&prep.payload_hash);
        }
        
        hasher.update(&payload_bytes);
        hasher.finalize().to_vec()
    }
}

fn main() {
    let mut consensus = MsgConsensus::new(21);
    let payload = b"Cross-chain message payload".to_vec();
    
    match consensus.prepare(payload) {
        Ok(_) => println!("Preparation successful"),
        Err(e) => println!("Preparation failed: {}", e),
    }

    match consensus.pre_commit() {
        Ok(_) => println!("Pre-commitment successful"),
        Err(e) => println!("Pre-commitment failed: {}", e),
    }

    match consensus.commit() {
        Ok(_) => println!("Commitment successful"),
        Err(e) => println!("Commitment failed: {}", e),
    }

    match consensus.execute() {
        Ok(_) => println!("Execution successful"),
        Err(e) => println!("Execution failed: {}", e),
    }
}