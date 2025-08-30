# Migration Guide: From Baton to SvelteKit Webhook Relay

This guide outlines how to migrate from the current Baton webhook relay implementation to a fully integrated SvelteKit application.

## 🔍 Current vs. New Architecture

### Current Implementation (Baton)
- **Dual Server Setup**: Separate Hono servers for ingestion (4000) and relay (4200)
- **WebSocket Communication**: Bidirectional WebSocket connections
- **Manual Client Management**: Map-based WebSocket client tracking
- **Bun Runtime**: Specific to Bun.js runtime environment

### New SvelteKit Implementation
- **Unified Application**: Single SvelteKit app with server routes
- **Server-Sent Events**: Unidirectional real-time communication
- **Store-based State**: Svelte stores for client-side state management
- **Platform Agnostic**: Deployable to various platforms (Vercel, Netlify, etc.)

## 📋 Migration Steps

### 1. Database Migration

The database schema remains largely compatible. Key changes:

```sql
-- Add indexes for better performance
CREATE INDEX idx_webhook_events_user_created ON "WebhookEvent"("userId", "createdAt");
CREATE INDEX idx_relay_targets_user_active ON "RelayTarget"("userId", "active");
```

**Migration Command**:
```bash
# Backup existing data
pg_dump your_database > backup.sql

# Apply new schema
npx prisma db push
```

### 2. Environment Variables

Update your environment configuration:

```env
# Remove Bun-specific variables
# Add SvelteKit-specific variables
AUTH_SECRET="your-auth-secret"
GITHUB_CLIENT_ID="your-github-client-id" 
GITHUB_CLIENT_SECRET="your-github-client-secret"
REDIRECT_URL="https://yourdomain.com/dashboard"
```

### 3. Deployment Changes

#### From Bun Servers
```typescript
// OLD: server/index.ts
const ingestServer = Bun.serve(IngestHandler);
const relayServer = Bun.serve(RelayHandler);
```

#### To SvelteKit Routes
```typescript
// NEW: Automatic routing via file structure
src/routes/api/webhook/[subdomain]/+server.ts
src/routes/api/relay/events/+server.ts
```

### 4. Client Integration

#### Old WebSocket Client
```javascript
// OLD: Manual WebSocket management
const ws = new WebSocket(`ws://${subdomain}.localhost:3000`);
ws.onmessage = (event) => {
  console.log(JSON.parse(event.data));
};
```

#### New SSE Integration
```typescript
// NEW: Svelte store integration
import { webhookStore } from '$lib/stores/webhooks';
webhookStore.connect(); // Automatic SSE connection
```

## 🔄 Feature Mapping

| Baton Feature | SvelteKit Equivalent | Implementation |
|---------------|---------------------|----------------|
| Subdomain routing | Dynamic route params | `[subdomain]/+server.ts` |
| WebSocket relay | Server-Sent Events | `/api/relay/events` |
| Client tracking | SSE connection map | `$lib/server/relay.ts` |
| Auth middleware | SvelteKit hooks | `hooks.server.ts` |
| Webhook logging | Same Prisma models | Enhanced with indexes |

## ⚡ Performance Improvements

### 1. Connection Management
- **Before**: Manual WebSocket connection tracking
- **After**: Automatic SSE connection lifecycle management

### 2. Real-time Updates
- **Before**: Bidirectional WebSocket (unnecessary overhead)
- **After**: Unidirectional SSE (optimal for webhook relay)

### 3. Scalability
- **Before**: Single server instance limitations
- **After**: Serverless-ready, horizontal scaling

## 🎯 Enhanced Features

### 1. User Interface
- **Dashboard**: Real-time webhook monitoring
- **Event History**: Searchable webhook logs
- **Relay Management**: Visual target configuration

### 2. Developer Experience
- **Type Safety**: Full TypeScript integration
- **Hot Reload**: SvelteKit development server
- **File-based Routing**: Intuitive API structure

### 3. Production Ready
- **Multiple Deployment Options**: Vercel, Netlify, Cloudflare
- **Built-in Optimizations**: SvelteKit's production optimizations
- **Error Handling**: Comprehensive error boundaries

## 🔧 Advanced Customizations

### 1. Custom Authentication

Replace GitHub OAuth with custom auth:

```typescript
// src/lib/server/auth.ts
import Credentials from '@auth/core/providers/credentials';

export const { handle } = SvelteKitAuth({
  providers: [
    Credentials({
      credentials: {
        username: { label: "Username", type: "text" },
        password: { label: "Password", type: "password" }
      },
      async authorize(credentials) {
        // Custom auth logic
      }
    })
  ]
});
```

### 2. Webhook Transformation

Add webhook transformation middleware:

```typescript
// src/routes/api/webhook/[subdomain]/+server.ts
function transformWebhook(payload: any, transformRules: any[]) {
  // Apply transformation rules
  return transformedPayload;
}
```

### 3. Rate Limiting

Implement rate limiting for webhook endpoints:

```typescript
// src/lib/server/rateLimit.ts
export function rateLimit(userId: string, windowMs: number, maxRequests: number) {
  // Rate limiting logic
}
```

## 🚀 Deployment Strategies

### 1. Vercel (Recommended)
```bash
npm install @sveltejs/adapter-vercel
vercel deploy
```

**Advantages**:
- Automatic HTTPS and domain management
- Edge functions for webhook processing
- Built-in analytics and monitoring

### 2. Self-hosted with Docker
```dockerfile
FROM node:18-alpine
WORKDIR /app
COPY package*.json ./
RUN npm ci --only=production
COPY build build/
COPY package.json .
EXPOSE 3000
CMD ["node", "build"]
```

### 3. Cloudflare Workers
```bash
npm install @sveltejs/adapter-cloudflare
wrangler deploy
```

## 🔍 Testing Strategy

### 1. Webhook Testing
```bash
# Test webhook endpoint
curl -X POST https://yourdomain.com/api/webhook/your-subdomain \
  -H "Content-Type: application/json" \
  -d '{"test": true, "message": "Hello World"}'
```

### 2. SSE Testing
```javascript
// Browser console
const eventSource = new EventSource('/api/relay/events');
eventSource.onmessage = console.log;
```

## 📈 Monitoring & Analytics

### 1. Built-in Metrics
- Real-time event counts
- Connection status monitoring
- Relay target success rates

### 2. External Monitoring
- **Sentry**: Error tracking and performance monitoring
- **LogRocket**: User session replay
- **Vercel Analytics**: Performance insights

## 🛡️ Security Considerations

### 1. Webhook Verification
Implement signature verification for specific providers:

```typescript
// Example: Stripe webhook verification
import { createHmac } from 'crypto';

function verifyStripeSignature(payload: string, signature: string, secret: string) {
  const expectedSignature = createHmac('sha256', secret)
    .update(payload, 'utf8')
    .digest('hex');
  return `sha256=${expectedSignature}` === signature;
}
```

### 2. Rate Limiting
```typescript
// Implement per-user rate limiting
const rateLimiter = new Map<string, { count: number; resetTime: number }>();
```

### 3. Input Sanitization
```typescript
import { z } from 'zod';

const webhookSchema = z.object({
  // Define expected webhook structure
});
```

## 🔄 Migration Checklist

- [ ] Backup existing database
- [ ] Set up new SvelteKit environment
- [ ] Configure authentication (GitHub OAuth)
- [ ] Migrate database schema with indexes
- [ ] Update DNS/subdomain routing
- [ ] Test webhook endpoints
- [ ] Verify real-time functionality
- [ ] Update external service webhook URLs
- [ ] Monitor for issues post-migration

## 🎉 Benefits of Migration

1. **Better Developer Experience**: Hot reload, type safety, modern tooling
2. **Improved Performance**: SSE efficiency, built-in optimizations
3. **Enhanced UI/UX**: Modern dashboard with real-time updates
4. **Platform Flexibility**: Deploy anywhere SvelteKit is supported
5. **Maintainability**: Cleaner architecture, better separation of concerns
6. **Scalability**: Serverless-ready, horizontal scaling capabilities

## 🆘 Troubleshooting

### Common Issues

1. **SSE Connection Drops**
   - Check network connectivity
   - Verify authentication status
   - Review browser console for errors

2. **Webhook Not Received**
   - Verify subdomain routing
   - Check external service configuration
   - Review server logs

3. **Database Connection Issues**
   - Verify DATABASE_URL format
   - Check database server status
   - Review Prisma connection logs

### Debug Mode

Enable debug logging:
```env
DEBUG=true
LOG_LEVEL=debug
```

## 📞 Support

For issues or questions about the migration:
1. Check the troubleshooting section
2. Review SvelteKit documentation
3. Check Auth.js documentation for authentication issues