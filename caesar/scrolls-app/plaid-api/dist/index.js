"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
const express_1 = __importDefault(require("express"));
const cors_1 = __importDefault(require("cors"));
const dotenv_1 = __importDefault(require("dotenv"));
const plaid_1 = require("plaid");
dotenv_1.default.config();
const app = (0, express_1.default)();
const port = process.env.PORT || 3003;
// Middleware
app.use((0, cors_1.default)());
app.use(express_1.default.json());
// Initialize Plaid client
const configuration = new plaid_1.Configuration({
    basePath: process.env.PLAID_ENV === 'production'
        ? plaid_1.PlaidEnvironments.production
        : plaid_1.PlaidEnvironments.sandbox,
    baseOptions: {
        headers: {
            'PLAID-CLIENT-ID': process.env.PLAID_CLIENT_ID || '',
            'PLAID-SECRET': process.env.PLAID_SECRET || '',
            'Plaid-Version': '2020-09-14',
        },
    },
});
const plaidClient = new plaid_1.PlaidApi(configuration);
// Store user tokens (in production, use a proper database)
const userTokens = {};
// Create link token
app.post('/api/plaid/create_link_token', async (req, res) => {
    try {
        const { userId = 'default_user' } = req.body;
        const request = {
            user: {
                client_user_id: userId,
            },
            client_name: 'Caesar Token Wallet',
            products: ['auth', 'transactions'],
            country_codes: ['US'],
            language: 'en',
        };
        const response = await plaidClient.linkTokenCreate(request);
        res.json({
            link_token: response.data.link_token,
            expiration: response.data.expiration,
        });
    }
    catch (error) {
        console.error('Error creating link token:', error);
        res.status(500).json({
            error: 'Failed to create link token',
            details: error.response?.data || error.message,
        });
    }
});
// Exchange public token for access token
app.post('/api/plaid/exchange_public_token', async (req, res) => {
    try {
        const { public_token, userId = 'default_user' } = req.body;
        if (!public_token) {
            return res.status(400).json({ error: 'public_token is required' });
        }
        const response = await plaidClient.itemPublicTokenExchange({
            public_token,
        });
        const accessToken = response.data.access_token;
        const itemId = response.data.item_id;
        // Store access token (in production, encrypt and store in database)
        userTokens[userId] = accessToken;
        // Get account information
        const accountsResponse = await plaidClient.accountsGet({
            access_token: accessToken,
        });
        res.json({
            success: true,
            item_id: itemId,
            accounts: accountsResponse.data.accounts,
            message: 'Bank account connected successfully',
        });
    }
    catch (error) {
        console.error('Error exchanging token:', error);
        res.status(500).json({
            error: 'Failed to exchange token',
            details: error.response?.data || error.message,
        });
    }
});
// Get account balances
app.post('/api/plaid/accounts', async (req, res) => {
    try {
        const { userId = 'default_user' } = req.body;
        const accessToken = userTokens[userId];
        if (!accessToken) {
            return res.status(400).json({
                error: 'No bank account connected for this user'
            });
        }
        const response = await plaidClient.accountsGet({
            access_token: accessToken,
        });
        res.json({
            accounts: response.data.accounts,
            item: response.data.item,
        });
    }
    catch (error) {
        console.error('Error fetching accounts:', error);
        res.status(500).json({
            error: 'Failed to fetch accounts',
            details: error.response?.data || error.message,
        });
    }
});
// Get recent transactions
app.post('/api/plaid/transactions', async (req, res) => {
    try {
        const { userId = 'default_user', count = 50 } = req.body;
        const accessToken = userTokens[userId];
        if (!accessToken) {
            return res.status(400).json({
                error: 'No bank account connected for this user'
            });
        }
        const response = await plaidClient.transactionsSync({
            access_token: accessToken,
            count,
        });
        res.json({
            transactions: response.data.added,
            accounts: response.data.accounts,
            total_transactions: response.data.added.length,
        });
    }
    catch (error) {
        console.error('Error fetching transactions:', error);
        res.status(500).json({
            error: 'Failed to fetch transactions',
            details: error.response?.data || error.message,
        });
    }
});
// Initiate ACH transfer (requires additional Plaid products)
app.post('/api/plaid/transfer', async (req, res) => {
    try {
        const { userId = 'default_user', accountId, amount, description = 'Caesar Token Purchase' } = req.body;
        const accessToken = userTokens[userId];
        if (!accessToken) {
            return res.status(400).json({
                error: 'No bank account connected for this user'
            });
        }
        if (!accountId || !amount) {
            return res.status(400).json({
                error: 'accountId and amount are required'
            });
        }
        // This is a placeholder - actual implementation would depend on
        // your payment processor integration (Stripe, Circle, etc.)
        res.json({
            success: true,
            message: 'Transfer initiated successfully',
            transfer_id: `transfer_${Date.now()}`,
            amount,
            description,
            status: 'pending',
        });
    }
    catch (error) {
        console.error('Error initiating transfer:', error);
        res.status(500).json({
            error: 'Failed to initiate transfer',
            details: error.response?.data || error.message,
        });
    }
});
// Health check
app.get('/health', (req, res) => {
    res.json({
        status: 'ok',
        timestamp: new Date().toISOString(),
        plaid_env: process.env.PLAID_ENV || 'sandbox',
    });
});
app.listen(port, () => {
    console.log(`🚀 Plaid API server running on port ${port}`);
    console.log(`📊 Environment: ${process.env.PLAID_ENV || 'sandbox'}`);
    console.log(`🔗 Health check: http://localhost:${port}/health`);
});
//# sourceMappingURL=index.js.map