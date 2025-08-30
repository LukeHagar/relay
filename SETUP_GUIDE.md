# Setup Guide: SvelteKit Webhook Relay

This guide walks you through setting up the enhanced SvelteKit webhook relay application with proper webhook ingestion and WebSocket compatibility.

## 🚀 Quick Start

### 1. Prerequisites

- Node.js 18+ or Bun
- PostgreSQL database
- GitHub OAuth App (for authentication)

### 2. Installation

```bash
cd sveltekit-integration
npm install
```

### 3. Environment Setup

Copy the environment template:
```bash
cp .env.example .env
```

Configure your `.env` file:
```env
# Database
DATABASE_URL="postgresql://username:password@localhost:5432/webhook_relay"

# GitHub OAuth (create at https://github.com/settings/applications/new)
GITHUB_CLIENT_ID="your_github_client_id"
GITHUB_CLIENT_SECRET="your_github_client_secret"
AUTH_SECRET="your_32_character_secret_key_here"

# Application
REDIRECT_URL="http://localhost:5173/dashboard"
WS_PORT="4001"
```

### 4. Database Setup

```bash
# Generate Prisma client
npm run db:generate

# Push schema to database
npm run db:push
```

### 5. Development

Start both SvelteKit and WebSocket servers:
```bash
npm run dev:full
```

Or start them separately:
```bash
# Terminal 1: SvelteKit app
npm run dev

# Terminal 2: WebSocket server
npm run dev:ws
```

## 🔧 GitHub OAuth Setup

1. Go to [GitHub Developer Settings](https://github.com/settings/applications/new)
2. Create a new OAuth App with:
   - **Application name**: Webhook Relay
   - **Homepage URL**: `http://localhost:5173`
   - **Authorization callback URL**: `http://localhost:5173/auth/callback/github`
3. Copy the Client ID and Client Secret to your `.env` file

## 📡 Webhook Ingestion Features

### Supported Content Types

The application handles all major webhook formats:

1. **JSON** (`application/json`)
   - Standard REST API webhooks
   - GitHub, GitLab, Slack webhooks
   - Custom JSON payloads

2. **Form Data** (`application/x-www-form-urlencoded`)
   - Stripe webhooks
   - PayPal IPN
   - Traditional form submissions

3. **Multipart** (`multipart/form-data`)
   - File uploads with webhook data
   - Complex form submissions

4. **XML** (`application/xml`, `text/xml`)
   - SOAP webhooks
   - Legacy system integrations

5. **Plain Text** (`text/plain`)
   - Simple notification webhooks
   - Log-based webhooks

### Enhanced Security Features

- **Header Filtering**: Sensitive headers (Authorization, Cookie) are excluded from logs
- **Error Handling**: Graceful failure handling that doesn't break webhook senders
- **Rate Limiting Ready**: Infrastructure for implementing rate limiting
- **Input Validation**: Robust parsing with fallback to raw data

## 🌐 WebSocket Implementation

### Why WebSockets?

The implementation uses standard WebSockets for maximum compatibility:

- **Universal Support**: Works with all browsers and WebSocket clients
- **Bidirectional Communication**: Supports ping/pong for connection health
- **Standard Protocol**: Compatible with load balancers and proxies
- **Real-time Updates**: Instant webhook event delivery

### Connection Management

- **Automatic Reconnection**: Client automatically reconnects on connection loss
- **Health Monitoring**: Ping/pong mechanism keeps connections alive
- **User Isolation**: Each user's webhooks are only sent to their connections
- **Connection Cleanup**: Automatic cleanup of stale connections

## 🧪 Testing Your Setup

### 1. Basic Functionality Test

```bash
# Test webhook ingestion
node test-client.js your-subdomain http://localhost:5173
```

### 2. Manual Webhook Test

```bash
# Send a test webhook
curl -X POST http://localhost:5173/api/webhook/your-subdomain \
  -H "Content-Type: application/json" \
  -H "X-Test-Header: test-value" \
  -d '{"test": true, "message": "Hello from curl!"}'
```

### 3. WebSocket Connection Test

```javascript
// In browser console
const ws = new WebSocket('ws://localhost:4001?token=your-session-token');
ws.onmessage = (event) => console.log('Received:', JSON.parse(event.data));
ws.onopen = () => ws.send(JSON.stringify({type: 'ping'}));
```

## 🚀 Production Deployment

### 1. Environment Variables

Update for production:
```env
DATABASE_URL="your-production-database-url"
AUTH_SECRET="your-production-secret-min-32-chars"
GITHUB_CLIENT_ID="your-prod-github-client-id"
GITHUB_CLIENT_SECRET="your-prod-github-client-secret"
REDIRECT_URL="https://yourdomain.com/dashboard"
WS_PORT="4001"
```

### 2. Vercel Deployment

```bash
# Install Vercel adapter
npm install @sveltejs/adapter-vercel

# Update svelte.config.js
import adapter from '@sveltejs/adapter-vercel';

# Deploy
vercel deploy
```

**Note**: For Vercel, you'll need to deploy the WebSocket server separately (Railway, Render, etc.) since Vercel doesn't support persistent WebSocket connections.

### 3. Self-hosted Deployment

```bash
# Build the application
npm run build

# Start production server
node build/index.js &

# Start WebSocket server
WS_PORT=4001 node scripts/websocket-server.js &
```

### 4. Docker Deployment

```dockerfile
FROM node:18-alpine

WORKDIR /app
COPY package*.json ./
RUN npm ci --only=production

COPY . .
RUN npm run build

EXPOSE 5173 4001

CMD ["sh", "-c", "node build/index.js & node scripts/websocket-server.js"]
```

## 🔧 Advanced Configuration

### 1. Custom Subdomain Routing

For production with custom domains, configure your reverse proxy:

```nginx
# Nginx configuration
server {
    server_name *.yourdomain.com;
    
    location /api/webhook/ {
        proxy_pass http://sveltekit-app;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
    
    location /ws {
        proxy_pass http://websocket-server:4001;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
    }
}
```

### 2. Database Optimization

Add indexes for better performance:
```sql
CREATE INDEX CONCURRENTLY idx_webhook_events_user_created 
ON "WebhookEvent"("userId", "createdAt" DESC);

CREATE INDEX CONCURRENTLY idx_relay_targets_user_active 
ON "RelayTarget"("userId", "active") WHERE "active" = true;
```

### 3. Monitoring Setup

Add health check endpoints:
```typescript
// src/routes/health/+server.ts
export const GET = async () => {
  const dbHealth = await checkDatabaseConnection();
  const wsHealth = getWebSocketServerStatus();
  
  return json({
    status: 'healthy',
    database: dbHealth,
    websocket: wsHealth,
    timestamp: new Date().toISOString()
  });
};
```

## 🐛 Troubleshooting

### Common Issues

1. **WebSocket Connection Fails**
   ```bash
   # Check if WebSocket server is running
   netstat -an | grep 4001
   
   # Test WebSocket endpoint
   wscat -c ws://localhost:4001?token=test
   ```

2. **Webhook Not Received**
   ```bash
   # Test webhook endpoint directly
   curl -v -X POST http://localhost:5173/api/webhook/test-subdomain \
     -H "Content-Type: application/json" \
     -d '{"test": true}'
   ```

3. **Database Connection Issues**
   ```bash
   # Test database connection
   npx prisma db pull
   
   # Reset database if needed
   npx prisma migrate reset
   ```

### Debug Mode

Enable detailed logging:
```env
NODE_ENV=development
DEBUG=webhook-relay:*
LOG_LEVEL=debug
```

## 📊 Performance Monitoring

### Key Metrics to Monitor

1. **Webhook Ingestion**:
   - Response time < 100ms
   - Success rate > 99.9%
   - Payload size handling

2. **WebSocket Performance**:
   - Connection count
   - Message delivery latency
   - Connection stability

3. **Database Performance**:
   - Query execution time
   - Connection pool usage
   - Storage growth

### Monitoring Tools

- **Application**: Built-in dashboard metrics
- **Infrastructure**: Prometheus + Grafana
- **Logs**: Winston + ELK stack
- **Errors**: Sentry integration

## 🎯 Next Steps

After setup, you can:

1. **Configure External Services**: Point GitHub, Stripe, etc. to your webhook endpoints
2. **Add Relay Targets**: Forward webhooks to your internal services
3. **Monitor Events**: Use the real-time dashboard
4. **Scale Up**: Deploy to production with load balancing

## 🆘 Support

If you encounter issues:

1. Check the troubleshooting section above
2. Review the console logs for both SvelteKit and WebSocket servers
3. Test individual components (database, auth, webhooks) separately
4. Use the built-in test suite to verify functionality

The application is designed to be robust and handle edge cases gracefully, ensuring reliable webhook processing in production environments.