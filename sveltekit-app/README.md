# Webhook Relay - SvelteKit Integration

A fullstack SvelteKit application that provides a modern web interface for the webhook relay system. This application integrates with the existing Bun-based webhook relay backend to provide real-time webhook management, forwarding, and monitoring.

## Features

### 🚀 Real-time Webhook Management
- **Live Dashboard**: Real-time webhook event monitoring with WebSocket connections
- **Event History**: Complete webhook event history with search and filtering
- **Event Details**: View full request bodies, headers, and metadata

### 🔄 Webhook Relay System
- **Multiple Targets**: Configure multiple forwarding destinations
- **Target Management**: Add, edit, delete, and toggle relay targets
- **Automatic Forwarding**: Incoming webhooks are automatically forwarded to all active targets

### 🔐 Authentication & Security
- **GitHub OAuth**: Secure authentication using GitHub
- **User Isolation**: Each user has their own subdomain and isolated data
- **Session Management**: Secure session handling with Auth.js

### 📊 Analytics & Monitoring
- **Statistics Dashboard**: View total events, recent activity, and success rates
- **Connection Status**: Real-time WebSocket connection monitoring
- **Event Filtering**: Filter events by HTTP method, search terms, and date ranges

## Architecture

### Frontend (SvelteKit)
- **Port**: 3000
- **Framework**: SvelteKit with TypeScript
- **Styling**: Tailwind CSS
- **Icons**: Lucide Svelte
- **State Management**: Svelte stores for real-time updates

### Backend Integration
- **Ingest Server**: Receives webhooks via subdomains (port 4000)
- **Relay Server**: Handles authentication and WebSocket connections (port 4200)
- **Database**: PostgreSQL with Prisma ORM

### Key Components

1. **WebSocket Client** (`src/lib/websocket.ts`)
   - Manages real-time connections to the relay server
   - Handles automatic reconnection
   - Provides reactive stores for webhook events

2. **Authentication** (`src/lib/auth.ts`)
   - GitHub OAuth integration
   - Session management with Auth.js
   - User data persistence

3. **Database Layer** (`src/lib/db.ts`)
   - Prisma client configuration
   - Connection pooling and optimization

## Getting Started

### Prerequisites
- Node.js 18+ or Bun
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

5. **Start the backend servers** (in separate terminals):
   ```bash
   # Terminal 1 - Ingest Server
   bun run server/index.ts
   
   # Terminal 2 - Relay Server  
   bun run server/relay.ts
   ```

### Usage

1. **Access the application**: http://localhost:3000
2. **Sign in with GitHub**: Click "Sign In with GitHub"
3. **Configure your subdomain**: Set up your unique subdomain for webhook reception
4. **Add relay targets**: Configure where webhooks should be forwarded
5. **Monitor webhooks**: View real-time webhook events on the dashboard

## API Endpoints

### Webhook Statistics
- `GET /api/webhooks/stats` - Get webhook statistics for dashboard

### Relay Targets
- `POST /api/targets` - Create a new relay target
- `PUT /api/targets/[id]` - Update a relay target
- `DELETE /api/targets/[id]` - Delete a relay target
- `PUT /api/targets/[id]/toggle` - Toggle target active status

## Webhook Flow

1. **Webhook Reception**: Incoming webhooks are received at `{subdomain}.yourdomain.com`
2. **Event Storage**: Webhook data is stored in the database
3. **Real-time Updates**: WebSocket clients receive immediate updates
4. **Target Forwarding**: Webhooks are forwarded to all active relay targets
5. **UI Updates**: SvelteKit interface updates in real-time

## Development

### Project Structure
```
sveltekit-app/
├── src/
│   ├── lib/           # Shared utilities and configurations
│   ├── routes/        # SvelteKit routes and API endpoints
│   └── app.css        # Global styles
├── prisma/           # Database schema and migrations
├── static/           # Static assets
└── package.json      # Dependencies and scripts
```

### Key Technologies
- **SvelteKit**: Fullstack framework for the web interface
- **TypeScript**: Type-safe development
- **Tailwind CSS**: Utility-first styling
- **Prisma**: Database ORM and migrations
- **Auth.js**: Authentication and session management
- **WebSocket**: Real-time communication

### Customization

#### Adding New Webhook Providers
1. Extend the webhook event schema in `prisma/schema.prisma`
2. Add provider-specific parsing in the ingest server
3. Update the UI components to display new fields

#### Custom Relay Logic
1. Modify the relay server to add custom forwarding logic
2. Add conditional forwarding based on webhook content
3. Implement retry mechanisms and error handling

#### UI Enhancements
1. Add new dashboard widgets for specific metrics
2. Create custom event visualizations
3. Implement advanced filtering and search

## Deployment

### Production Setup
1. **Build the application**:
   ```bash
   npm run build
   ```

2. **Set up production environment**:
   - Configure production database
   - Set up proper domain and SSL certificates
   - Configure reverse proxy for subdomain routing

3. **Deploy with adapter**:
   ```bash
   npm run preview
   ```

### Environment Variables for Production
```env
DATABASE_URL="postgresql://..."
AUTH_SECRET="production-secret"
GITHUB_ID="production-github-id"
GITHUB_SECRET="production-github-secret"
REDIRECT_URL="https://yourdomain.com"
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## License

MIT License - see LICENSE file for details