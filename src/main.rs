use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

// Define a simple Validator structure
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Validator {
    id: String,
    stake: u64,
    performance_score: f64,
    is_byzantine: bool,
}

// Consensus Phases
enum ConsensusPhase {
    Preparation,
    PreCommitment,
    Commitment,
}

// Message structures
#[derive(Clone, Debug, Serialize, Deserialize)]
struct PreparationMessage {
    validator_id: String,
    payload_hash: String,
    signature: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PreCommitmentMessage {
    validator_id: String,
    preparation_hash: String,
    proposed_block: u64,
    signature: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CommitmentMessage {
    validator_id: String,
    final_payload_hash: String,
    execution_block: u64,
    signature: String,
}

// Simulate Validator selection based on stake and performance
fn select_committee(validators: &Vec<Validator>, committee_size: usize) -> Vec<Validator> {
    let mut selected_committee = validators.clone();
    selected_committee.sort_by(|a, b| {
        (b.stake as f64 * b.performance_score)
            .partial_cmp(&(a.stake as f64 * a.performance_score))
            .unwrap()
    });
    selected_committee.truncate(committee_size);
    selected_committee
}

// Simulate the consensus process
async fn run_consensus_round(committee: Vec<Validator>, payload_hash: &str) {
    let mut current_phase = ConsensusPhase::Preparation;
    let (tx, mut rx) = mpsc::channel::<String>(committee.len());

    // Preparation Phase
    if let ConsensusPhase::Preparation = current_phase {
        println!("Starting Preparation Phase...");
        for validator in &committee {
            let tx = tx.clone();
            let payload_hash = payload_hash.to_string();
            let validator_clone = validator.clone();  // Clone the validator
        
            tokio::spawn(async move {
                // Simulate validation
                sleep(Duration::from_millis(100)).await;
                let message = PreparationMessage {
                    validator_id: validator_clone.id.clone(),
                    payload_hash,
                    signature: "signature_placeholder".to_string(),
                };
                tx.send(serde_json::to_string(&message).unwrap()).await.unwrap();
            });
        }
        current_phase = ConsensusPhase::PreCommitment;
    }


    // Collect Preparation Messages
    let mut preparation_messages = Vec::new();
    while let Some(message) = rx.recv().await {
        preparation_messages.push(message);
        if preparation_messages.len() >= (2 * (committee.len() / 3) + 1) {
            break;
        }
    }
    println!("Collected Preparation Messages: {:?}", preparation_messages);

    // Pre-Commitment Phase
    if let ConsensusPhase::PreCommitment = current_phase {
        println!("Starting Pre-Commitment Phase...");
        for validator in &committee {
            let tx = tx.clone();
            let preparation_hash = "preparation_hash_placeholder".to_string();
            let proposed_block = 123456; // Simulated block number
            let validator_clone = validator.clone();
            tokio::spawn(async move {
                // Simulate pre-commitment
                sleep(Duration::from_millis(100)).await;
                let message = PreCommitmentMessage {
                    validator_id: validator_clone.id.clone(),
                    preparation_hash,
                    proposed_block,
                    signature: "signature_placeholder".to_string(),
                };
                tx.send(serde_json::to_string(&message).unwrap()).await.unwrap();
            });
        }
        current_phase = ConsensusPhase::Commitment;
    }


    // Collect Pre-Commitment Messages
    let mut pre_commitment_messages = Vec::new();
    while let Some(message) = rx.recv().await {
        pre_commitment_messages.push(message);
        if pre_commitment_messages.len() >= (2 * (committee.len() / 3) + 1) {
            break;
        }
    }
    println!("Collected Pre-Commitment Messages: {:?}", pre_commitment_messages);

    // Commitment Phase
    if let ConsensusPhase::Commitment = current_phase {
        println!("Starting Commitment Phase...");
        for validator in &committee {
            let tx = tx.clone();
            let final_payload_hash = "final_payload_hash_placeholder".to_string();
            let execution_block = 123456; // Simulated execution block
            let validator_clone = validator.clone();
            tokio::spawn(async move {
                // Simulate commitment
                sleep(Duration::from_millis(100)).await;
                let message = CommitmentMessage {
                    validator_id: validator_clone.id.clone(),
                    final_payload_hash,
                    execution_block,
                    signature: "signature_placeholder".to_string(),
                };
                tx.send(serde_json::to_string(&message).unwrap()).await.unwrap();
            });
        }
    }

    // Collect Commitment Messages
    let mut commitment_messages = Vec::new();
    while let Some(message) = rx.recv().await {
        commitment_messages.push(message);
        if commitment_messages.len() >= (2 * (committee.len() / 3) + 1) {
            break;
        }
    }
    println!("Collected Commitment Messages: {:?}", commitment_messages);

    // Finalize Consensus
    println!("Consensus Achieved! Executing Payload...");
}

#[tokio::main]
async fn main() {
    // Sample Validators
    let validators = vec![
        Validator { id: "Validator1".to_string(), stake: 15000, performance_score: 0.9, is_byzantine: false },
        Validator { id: "Validator2".to_string(), stake: 20000, performance_score: 0.95, is_byzantine: false },
        Validator { id: "Validator3".to_string(), stake: 12000, performance_score: 0.85, is_byzantine: false },
        Validator { id: "Validator4".to_string(), stake: 25000, performance_score: 0.97, is_byzantine: false },
        Validator { id: "Validator5".to_string(), stake: 18000, performance_score: 0.88, is_byzantine: false },
        Validator { id: "Validator6".to_string(), stake: 16000, performance_score: 0.92, is_byzantine: false },
        Validator { id: "Validator7".to_string(), stake: 14000, performance_score: 0.87, is_byzantine: false },
    ];

    // Select Committee
    let committee_size = 7;
    let committee = select_committee(&validators, committee_size);

    // Run Consensus Round
    run_consensus_round(committee, "payload_hash_example").await;
}
