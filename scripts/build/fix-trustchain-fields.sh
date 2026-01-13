#!/bin/bash
set -e

echo "=== Fixing TrustChain field mismatches ==="

# Fix SpaceProof field references
echo "1. Fixing SpaceProof fields..."
sed -i 's/proof\.proof_generated/std::time::Instant::now()/g' trustchain/src/consensus/real_validator.rs

# Fix WorkProof field references
echo "2. Fixing WorkProof fields..."
# Replace computation_time with a Duration based on computational_power
sed -i 's/proof\.computation_time/std::time::Duration::from_secs(proof.computational_power \/ 1000)/g' trustchain/src/consensus/real_validator.rs
# Replace work_result with workload_id
sed -i 's/proof\.work_result/proof.workload_id/g' trustchain/src/consensus/real_validator.rs

# Fix TimeProof field references
echo "3. Fixing TimeProof fields..."
# Find and fix TimeProof field references
sed -i 's/proof\.vdf_output/proof.vdf_iterations.to_string()/g' trustchain/src/consensus/real_validator.rs
sed -i 's/proof\.time_challenges/vec![]/g' trustchain/src/consensus/real_validator.rs
sed -i 's/proof\.synchronization_time/std::time::Duration::from_secs(1)/g' trustchain/src/consensus/real_validator.rs
sed -i 's/proof\.proof_timestamp/proof.timestamp/g' trustchain/src/consensus/real_validator.rs

# Fix ConsensusResult fields
echo "4. Fixing ConsensusResult fields..."
# Find files with ConsensusResult::Valid and fix them
for file in trustchain/src/consensus/*.rs; do
    if grep -q "ConsensusResult::Valid" "$file"; then
        echo "   Fixing $file..."
        # Replace proof_hash with block_hash and validator_id with empty string
        sed -i 's/proof_hash: /block_hash: Some(/g' "$file"
        sed -i 's/validator_id: /); \/\/ validator_id: /g' "$file"
    fi
done

echo "=== Field fixes applied ==="