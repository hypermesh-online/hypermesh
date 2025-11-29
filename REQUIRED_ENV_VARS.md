# Required Environment Variables

**CRITICAL: All AWS credentials and sensitive data have been removed from the codebase. These MUST be provided via environment variables.**

## AWS Infrastructure

### Core AWS Configuration
- `AWS_REGION` - AWS region for deployment (e.g., "us-east-1")
- `AWS_ACCESS_KEY_ID` - AWS access key (use IAM roles when possible)
- `AWS_SECRET_ACCESS_KEY` - AWS secret access key (use IAM roles when possible)

### S3 Storage
- `AWS_S3_CT_LOGS_BUCKET` - S3 bucket for Certificate Transparency logs
- `AWS_S3_BACKUP_BUCKET` - S3 bucket for backups
- `S3_BUCKET_NAME` - General purpose S3 bucket

### KMS Encryption
- `AWS_KMS_KEY_ARN` - KMS key ARN for encryption (format: arn:aws:kms:region:account:key/id)

### Infrastructure
- `AWS_LOAD_BALANCER_SSL_CERT_ARN` - ACM certificate ARN for load balancer SSL
- `TF_STATE_BUCKET` - Terraform state bucket name
- `TF_LOCK_TABLE` - DynamoDB table for Terraform state locks

## Test Environment
- `TEST_S3_BUCKET` - S3 bucket for test environment
- `TEST_AWS_REGION` - AWS region for tests
- `TEST_KMS_KEY_ARN` - KMS key for test environment
- `TEST_WALLET_PRIVATE_KEY` - Private key for test wallet (testnet only!)

## Blockchain RPC Endpoints
- `ETHEREUM_RPC_URL` - Ethereum mainnet RPC endpoint
- `ETH_TESTNET_RPC` - Ethereum testnet RPC endpoint
- `POLYGON_RPC_URL` - Polygon mainnet RPC endpoint
- `ARBITRUM_RPC_URL` - Arbitrum RPC endpoint
- `OPTIMISM_RPC_URL` - Optimism RPC endpoint
- `BASE_RPC_URL` - Base RPC endpoint

## API Keys
- `INFURA_API_KEY` - Infura project ID for blockchain access
- `ETHERSCAN_API_KEY` - Etherscan API key for contract verification
- `POLYGONSCAN_API_KEY` - Polygonscan API key

## Stripe Configuration
- `STRIPE_PUBLISHABLE_KEY` - Stripe publishable key
- `STRIPE_SECRET_KEY` - Stripe secret key
- `STRIPE_WEBHOOK_SECRET` - Stripe webhook secret
- `STRIPE_CONNECT_CLIENT_ID` - Stripe Connect client ID
- `STRIPE_PLATFORM_ACCOUNT_ID` - Stripe platform account ID

## JWT & Security
- `JWT_SECRET` - JWT signing secret
- `REFRESH_TOKEN_SECRET` - Refresh token secret

## LayerZero Configuration
- `LAYERZERO_ENDPOINT_V2` - LayerZero V2 endpoint address
- `GATE_TOKEN_CONTRACT` - Gate token contract address
- `USDC_CONTRACT` - USDC contract address

## Blockchain Keys (Production - Store in HSM/Vault)
- `DEPLOYER_PRIVATE_KEY` - Contract deployer private key
- `GATEWAY_OPERATOR_KEY` - Gateway operator private key
- `MNEMONIC` - HD wallet mnemonic phrase

## KYC/Compliance
- `JUMIO_API_TOKEN` - Jumio KYC API token
- `JUMIO_API_SECRET` - Jumio KYC API secret
- `SANCTIONS_SCREENING_API_KEY` - Sanctions screening API key

## Monitoring
- `SENTRY_DSN` - Sentry error tracking DSN

## Database
- `DATABASE_URL` - PostgreSQL connection string
- `REDIS_URL` - Redis connection string

## Security Best Practices

1. **Never commit credentials to version control**
2. **Use IAM roles instead of access keys when possible**
3. **Rotate credentials regularly**
4. **Use different credentials for each environment (dev/staging/prod)**
5. **Store production credentials in a secure vault (AWS Secrets Manager, HashiCorp Vault, etc.)**
6. **Enable MFA on all AWS accounts**
7. **Use least-privilege access policies**
8. **Monitor credential usage with CloudTrail**

## Example .env.local File Structure

```bash
# Create a .env.local file (gitignored) with your actual values:
cp .env.example .env.local
# Edit .env.local with your actual credentials
```

## Deployment Commands

```bash
# Load environment variables before deployment
source .env.local

# Or use dotenv for Node.js projects
npm run deploy

# For Terraform, use backend config
terraform init \
  -backend-config="bucket=${TF_STATE_BUCKET}" \
  -backend-config="region=${AWS_REGION}" \
  -backend-config="dynamodb_table=${TF_LOCK_TABLE}"
```

## Verification

Run the verification script to ensure no credentials remain in code:
```bash
./scripts/verify-no-credentials.sh
```

---

**⚠️ WARNING**: The codebase previously contained hardcoded credentials that have been removed. Ensure all deployments use proper environment variable configuration going forward.