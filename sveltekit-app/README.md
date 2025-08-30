# Webhook Relay - SvelteKit Fullstack Application

A complete webhook relay system built entirely with **Svelte 5**, **SvelteKit 3**, and **Tailwind 4**, providing webhook ingestion, relay functionality, and a modern web interface all in one application.

## Features

### 🚀 Complete Webhook Management
- **Webhook Ingestion**: Receive webhooks via subdomain routing
- **Real-time Dashboard**: Live webhook event monitoring with WebSocket connections
- **Event History**: Complete webhook event history with search and filtering
- **Relay System**: Forward webhooks to multiple configured targets

### 🔄 Webhook Relay System
- **Multiple Targets**: Configure multiple forwarding destinations
- **Target Management**: Add, edit, delete, and toggle relay targets
- **Automatic Forwarding**: Incoming webhooks are automatically forwarded to all active targets
- **Relay Analytics**: Track success rates and response times

### 🔐 Authentication & Security
- **GitHub OAuth**: Secure authentication using GitHub
- **User Isolation**: Each user has their own subdomain and isolated data
- **Session Management**: Secure session handling with Auth.js

### 📊 Analytics & Monitoring
- **Statistics Dashboard**: View total events, recent activity, and success rates
- **Connection Status**: Real-time WebSocket connection monitoring
- **Event Filtering**: Filter events by HTTP method, search terms, and date ranges

## Architecture

### Single SvelteKit Application
- **Webhook Ingestion**: `/webhook/[...path]` - Handles all incoming webhooks
- **WebSocket Server**: `/api/ws` - Real-time updates for authenticated users
- **Web Interface**: Modern dashboard and management UI with Svelte 5 signals
- **Database**: PostgreSQL with Prisma ORM

### Key Components

1. **Webhook Handler** (`src/routes/webhook/[...path]/+server.ts`)
   - Receives webhooks via subdomain routing
   - Stores events in database
   - Forwards to configured relay targets
   - Broadcasts real-time updates

2. **WebSocket Handler** (`src/routes/api/ws/+server.ts`)
   - Manages authenticated WebSocket connections
   - Provides real-time webhook event updates
   - Handles connection lifecycle

3. **Relay Service** (`src/lib/relay.ts`)
   - Forwards webhooks to configured targets
   - Tracks relay success/failure
   - Handles timeouts and errors

4. **Authentication** (`src/lib/auth.ts`)
   - GitHub OAuth integration
   - Session management
   - User data persistence

## Getting Started

### Prerequisites
- Node.js 20+ (for Svelte 5 and SvelteKit 3)
- PostgreSQL database
- GitHub OAuth application

### Installation

1. **Clone and install dependencies**:
   ```bash
   cd sveltekit-app
   npm install
   ```

2. **Set up environment variables**:
   ```bash
   cp .env.example .env
   ```
   
   Configure the following variables:
   ```env
   DATABASE_URL="postgresql://user:password@localhost:5432/webhook_relay"
   AUTH_SECRET="your-auth-secret"
   GITHUB_ID="your-github-oauth-app-id"
   GITHUB_SECRET="your-github-oauth-app-secret"
   REDIRECT_URL="http://localhost:3000"
   WEBHOOK_DOMAIN="yourdomain.com"
   ```

3. **Set up the database**:
   ```bash
   npx prisma generate
   npx prisma db push
   ```

4. **Start the development server**:
   ```bash
   npm run dev
   ```

### Usage

1. **Access the application**: http://localhost:3000
2. **Sign in with GitHub**: Click "Sign In with GitHub"
3. **Configure your subdomain**: Set up your unique subdomain for webhook reception
4. **Add relay targets**: Configure where webhooks should be forwarded
5. **Monitor webhooks**: View real-time webhook events on the dashboard

## Webhook Flow

### 1. Webhook Reception
```
External Service → https://{subdomain}.yourdomain.com/webhook → SvelteKit Handler
```

### 2. Processing Pipeline
1. **Subdomain Extraction**: Extract user subdomain from hostname
2. **User Validation**: Verify subdomain belongs to authenticated user
3. **Event Storage**: Store webhook data in database
4. **Target Relay**: Forward to all active relay targets
5. **Real-time Broadcast**: Send updates to connected WebSocket clients

### 3. Real-time Updates
```
Webhook Event → Database → WebSocket Broadcast → SvelteKit UI Updates
```

## API Endpoints

### Webhook Ingestion
- `ANY /webhook/[...path]` - Receive webhooks (subdomain-based routing)

### WebSocket
- `GET /api/ws` - WebSocket connection for real-time updates

### Webhook Management
- `GET /api/webhooks/stats` - Get webhook statistics for dashboard
- `POST /api/test-webhook` - Create test webhook events

### Relay Targets
- `POST /api/targets` - Create a new relay target
- `PUT /api/targets/[id]` - Update a relay target
- `DELETE /api/targets/[id]` - Delete a relay target
- `PUT /api/targets/[id]/toggle` - Toggle target active status

### User Settings
- `PUT /api/settings/subdomain` - Update user subdomain

## Development

### Project Structure
```
sveltekit-app/
├── src/
│   ├── lib/                    # Shared utilities and services
│   │   ├── auth.ts            # Authentication configuration
│   │   ├── db.ts              # Database connection
│   │   ├── relay.ts           # Webhook relay service
│   │   └── websocket.ts       # WebSocket client utilities
│   ├── routes/
│   │   ├── webhook/           # Webhook ingestion endpoints
│   │   ├── api/               # API endpoints
│   │   ├── +page.svelte       # Dashboard
│   │   ├── webhooks/          # Webhook management
│   │   ├── targets/           # Relay target management
│   │   └── settings/          # User settings
│   └── app.css                # Global styles
├── prisma/                    # Database schema and migrations
└── package.json               # Dependencies and scripts
```

### Key Technologies
- **Svelte 5**: Latest version with signals and runes
- **SvelteKit 3**: Fullstack framework for web interface and API
- **Tailwind 4**: Latest version with improved performance
- **TypeScript**: Type-safe development
- **Prisma**: Database ORM and migrations
- **Auth.js**: Authentication and session management
- **WebSocket**: Real-time communication

## Production Deployment

### 1. Build the Application
```bash
npm run build
```

### 2. Environment Configuration
```env
DATABASE_URL="postgresql://..."
AUTH_SECRET="production-secret"
GITHUB_ID="production-github-id"
GITHUB_SECRET="production-github-secret"
REDIRECT_URL="https://yourdomain.com"
WEBHOOK_DOMAIN="yourdomain.com"
```

### 3. Domain Configuration
- Set up wildcard DNS records (`*.yourdomain.com`)
- Configure reverse proxy (nginx/traefik) for subdomain routing
- Set up SSL certificates for secure webhook reception

### 4. Start Production Server
```bash
npm run preview
```

## Testing

### Test Webhook Creation
```bash
curl -X POST http://localhost:3000/api/test-webhook \
  -H "Content-Type: application/json" \
  -d '{"subdomain": "your-subdomain", "method": "POST", "path": "/test", "body": {"test": "data"}}'
```

### Send Webhook to Your Endpoint
```bash
curl -X POST https://your-subdomain.yourdomain.com/webhook/test \
  -H "Content-Type: application/json" \
  -d '{"event": "test", "data": "example"}'
```

## Security Considerations

### 1. Authentication & Authorization
- GitHub OAuth for user authentication
- Session-based authentication with secure cookies
- User data isolation in database queries
- CSRF protection via SvelteKit

### 2. Data Protection
- Input validation on all endpoints
- SQL injection prevention via Prisma ORM
- XSS protection via SvelteKit's built-in sanitization
- Secure WebSocket connections

### 3. Rate Limiting
- Consider implementing rate limiting for webhook endpoints
- WebSocket connection limits
- Database query optimization

## Monitoring & Analytics

### 1. Real-time Metrics
- WebSocket connection status
- Webhook event counts
- Relay target success rates
- User activity tracking

### 2. Error Tracking
- WebSocket connection failures
- API endpoint errors
- Database connection issues
- Relay target failures

## Future Enhancements

### 1. Advanced Features
- **Webhook Templates**: Pre-configured webhook formats
- **Conditional Relay**: Forward based on webhook content
- **Retry Mechanisms**: Automatic retry for failed relays
- **Webhook Signatures**: Security verification

### 2. Enterprise Features
- **Team Management**: Multi-user organizations
- **API Keys**: Programmatic access
- **Audit Logs**: Complete activity tracking
- **Advanced Analytics**: Detailed performance metrics

## License

MIT License - see LICENSE file for details