# Plaid Integration Setup Guide

This guide will help you set up Plaid bank account integration for the Caesar Token wallet.

## Prerequisites

1. **Plaid Developer Account**: Sign up at [dashboard.plaid.com](https://dashboard.plaid.com)
2. **Node.js**: Version 16 or higher
3. **npm**: Version 7 or higher

## Step 1: Plaid Dashboard Setup

### 1.1 Create Plaid Account
1. Visit [dashboard.plaid.com](https://dashboard.plaid.com)
2. Sign up for a developer account
3. Complete the verification process

### 1.2 Get API Keys
1. In your Plaid dashboard, navigate to **Team Settings > Keys**
2. Copy your **Client ID** and **Secret** for Sandbox environment
3. Note: For production, you'll need to apply for production access

### 1.3 Configure Allowed Redirect URIs
1. Go to **Team Settings > API**
2. Add your frontend URL: `http://localhost:3002`
3. Add your backend URL: `http://localhost:3003`

## Step 2: Backend Setup

### 2.1 Navigate to Backend Directory
```bash
cd scrolls-app/plaid-api
```

### 2.2 Install Dependencies
```bash
npm install
```

### 2.3 Configure Environment Variables
1. Copy the example environment file:
   ```bash
   cp .env.example .env
   ```

2. Edit `.env` file with your Plaid credentials:
   ```bash
   # Plaid Configuration
   PLAID_CLIENT_ID=your_client_id_from_plaid_dashboard
   PLAID_SECRET=your_sandbox_secret_from_plaid_dashboard
   PLAID_ENV=sandbox

   # Server Configuration
   PORT=3003
   ```

### 2.4 Start Backend Server
```bash
# Development mode with auto-reload
npm run dev

# Or build and run production
npm run build
npm start
```

The backend server will start on `http://localhost:3003`

## Step 3: Frontend Setup

### 3.1 Navigate to Frontend Directory
```bash
cd scrolls-app/satchel-wallet
```

### 3.2 Configure Environment Variables
1. Copy the example environment file:
   ```bash
   cp .env.example .env
   ```

2. Edit `.env` file:
   ```bash
   REACT_APP_API_BASE_URL=http://localhost:3003
   REACT_APP_PLAID_ENV=sandbox
   ```

### 3.3 Start Frontend Development Server
```bash
npm run dev
```

The frontend will start on `http://localhost:3002`

## Step 4: Testing the Integration

### 4.1 Access the Wallet
1. Open your browser and navigate to `http://localhost:3002`
2. Click on the **Bank Accounts** tab in the navigation

### 4.2 Connect a Test Bank Account
1. Click **Connect Bank Account**
2. In the Plaid Link interface:
   - Select any bank from the list
   - Use these test credentials:
     - Username: `user_good`
     - Password: `pass_good`
   - Complete the connection flow

### 4.3 Test Features
- **View Accounts**: See connected bank accounts with balances (toggle show/hide)
- **Load Transactions**: Click to view recent transaction history
- **Transfer**: Select an account and enter amount to simulate a transfer

## Step 5: API Endpoints

Your backend provides these endpoints:

### Create Link Token
```bash
POST /api/plaid/create_link_token
Content-Type: application/json

{
  "userId": "default_user"
}
```

### Exchange Public Token
```bash
POST /api/plaid/exchange_public_token
Content-Type: application/json

{
  "public_token": "public-sandbox-xxxx",
  "userId": "default_user"
}
```

### Get Accounts
```bash
POST /api/plaid/accounts
Content-Type: application/json

{
  "userId": "default_user"
}
```

### Get Transactions
```bash
POST /api/plaid/transactions
Content-Type: application/json

{
  "userId": "default_user",
  "count": 50
}
```

### Initiate Transfer
```bash
POST /api/plaid/transfer
Content-Type: application/json

{
  "userId": "default_user",
  "accountId": "account_id_here",
  "amount": 100.00,
  "description": "Caesar Token Purchase"
}
```

## Step 6: Production Considerations

### 6.1 Database Integration
- Replace in-memory token storage with encrypted database storage
- Implement proper user authentication and session management
- Store access tokens securely with encryption

### 6.2 Security Enhancements
- Implement rate limiting
- Add request validation and sanitization
- Use HTTPS in production
- Implement proper error handling without exposing sensitive information

### 6.3 Plaid Production Access
1. Apply for production access in your Plaid dashboard
2. Complete compliance requirements
3. Update environment variables to use production endpoints
4. Test with real bank accounts in sandbox mode first

### 6.4 Environment Variables for Production
```bash
# Production Plaid Configuration
PLAID_CLIENT_ID=your_production_client_id
PLAID_SECRET=your_production_secret
PLAID_ENV=production

# Security
JWT_SECRET=your_secure_jwt_secret
DATABASE_URL=your_encrypted_database_url

# Server
PORT=3003
NODE_ENV=production
```

## Troubleshooting

### Common Issues

1. **"Configuration not available" Error**
   - Ensure backend server is running on port 3003
   - Check that environment variables are set correctly
   - Verify Plaid credentials are valid

2. **CORS Errors**
   - Ensure frontend URL is added to Plaid dashboard allowed origins
   - Check that backend CORS is configured for your frontend URL

3. **Token Exchange Failures**
   - Verify Plaid secret key matches your environment (sandbox/production)
   - Check that public_token is being passed correctly from frontend

4. **Network Connection Issues**
   - Ensure both frontend and backend servers are running
   - Check that API_BASE_URL in frontend matches backend port
   - Verify firewall settings allow connections on ports 3002 and 3003

### Test Credentials for Sandbox

Use these credentials in Plaid's sandbox environment:

**Successful Connection:**
- Username: `user_good`
- Password: `pass_good`

**Requires MFA:**
- Username: `user_good`  
- Password: `pass_good`
- MFA Code: `1234`

**Account Locked:**
- Username: `user_locked`
- Password: `pass_good`

## Support

- **Plaid Documentation**: https://plaid.com/docs/
- **Plaid API Reference**: https://plaid.com/docs/api/
- **React Plaid Link**: https://github.com/plaid/react-plaid-link

## Security Note

⚠️ **Important**: Never commit your `.env` files to version control. The example files are provided for configuration reference only. Always use environment variables for sensitive credentials in production.