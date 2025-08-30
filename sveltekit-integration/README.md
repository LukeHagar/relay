# Webhook Relay - Modern SvelteKit Implementation

A cutting-edge webhook relay system built with **Svelte 5**, **SvelteKit 2**, and **Tailwind 4** - featuring runes, enhanced reactivity, and modern UI patterns.

## ✨ Modern Stack Features

### 🎯 **Svelte 5 Enhancements**
- **Runes System**: `$state`, `$derived`, `$effect` for superior reactivity
- **Enhanced Performance**: Faster updates and better memory management  
- **Type Safety**: Full TypeScript integration with modern syntax
- **Snippet System**: Reusable template fragments

### 🚀 **SvelteKit 2 Features**
- **Enhanced Routing**: Improved file-based routing with better performance
- **Modern API**: Updated event object patterns and error handling
- **Better Dev Experience**: Faster HMR and improved debugging
- **Production Optimizations**: Enhanced build system and deployment

### 🎨 **Tailwind 4 Integration**
- **Vite Plugin**: Native Tailwind 4 integration via `@tailwindcss/vite`
- **Enhanced Color System**: Modern color palettes with semantic naming
- **Improved Animations**: Smooth transitions and micro-interactions
- **CSS-in-JS Patterns**: Theme function integration

## 🚀 Core Features

- **Real-time Monitoring**: Live webhook events using standard WebSockets
- **Universal Webhook Support**: JSON, form data, XML, plain text, multipart
- **Subdomain Routing**: Each user gets a unique subdomain for webhook endpoints
- **Relay Targets**: Forward webhooks to multiple destinations with retry logic
- **Modern Authentication**: GitHub OAuth with Auth.js integration
- **Advanced UI**: Reactive dashboard with real-time metrics and notifications
- **Production Ready**: Comprehensive error handling and monitoring

## 🏗️ Architecture

### Key Improvements over Original Implementation

1. **Unified Application**: Single SvelteKit app instead of dual server setup
2. **Server-Sent Events**: More efficient than WebSockets for one-way communication
3. **File-based Routing**: Leverages SvelteKit's intuitive routing system
4. **Type Safety**: Full TypeScript integration throughout
5. **Modern Auth**: Auth.js integration with proper session management

### Directory Structure

```
src/
├── lib/
│   ├── components/           # Reusable Svelte components
│   ├── server/              # Server-side utilities
│   │   ├── auth.ts          # Authentication configuration
│   │   ├── db.ts            # Database connection
│   │   └── relay.ts         # Webhook relay logic
│   └── stores/              # Client-side state management
│       └── webhooks.ts      # Webhook-related stores
├── routes/
│   ├── api/
│   │   ├── webhook/[subdomain]/  # Webhook ingestion endpoint
│   │   ├── relay/               # Relay management APIs
│   │   └── webhooks/            # Webhook history API
│   ├── auth/                    # Authentication routes
│   └── dashboard/               # Protected dashboard pages
└── app.d.ts                     # TypeScript declarations
```

## 🛠️ Setup

1. **Install Dependencies**:
   ```bash
   npm install
   ```

2. **Environment Variables**:
   Copy `.env.example` to `.env` and configure:
   ```env
   DATABASE_URL="postgresql://..."
   AUTH_SECRET="your-secret"
   GITHUB_CLIENT_ID="your-github-app-id"
   GITHUB_CLIENT_SECRET="your-github-app-secret"
   ```

3. **Database Setup**:
   ```bash
   npx prisma db push
   npx prisma generate
   ```

4. **Development**:
   ```bash
   npm run dev
   ```

## 📡 API Endpoints

### Webhook Ingestion
- `POST /api/webhook/{subdomain}` - Receive webhooks for a specific user

### Relay Management
- `GET /api/relay/events` - Server-Sent Events stream for real-time updates
- `GET /api/relay/targets` - List user's relay targets
- `POST /api/relay/targets` - Add new relay target
- `DELETE /api/relay/targets` - Remove relay target

### Webhook History
- `GET /api/webhooks` - Get webhook event history

## 🔄 Real-time Communication

The system uses **Server-Sent Events (SSE)** instead of WebSockets for several advantages:

1. **Simpler Implementation**: No need for bidirectional communication
2. **Better Browser Support**: Native EventSource API
3. **Automatic Reconnection**: Built-in reconnection logic
4. **HTTP/2 Multiplexing**: Better performance over HTTP/2

### SSE Connection Flow

1. Client connects to `/api/relay/events`
2. Server maintains connection map by user ID
3. Incoming webhooks trigger real-time broadcasts
4. Automatic heartbeat keeps connections alive

## 🎯 Usage Example

1. **Sign in** with GitHub
2. **Get your webhook endpoint**: `https://yourdomain.com/api/webhook/{your-subdomain}`
3. **Configure external services** to send webhooks to your endpoint
4. **Add relay targets** to forward webhooks to your services
5. **Monitor events** in real-time through the dashboard

## 🔒 Security Features

- **Authentication Required**: All dashboard routes protected
- **User Isolation**: Webhooks isolated by subdomain/user
- **Header Filtering**: Sensitive headers excluded from logs
- **Input Validation**: Zod schemas for API validation

## 🚀 Deployment

### Vercel (Recommended)
```bash
npm install @sveltejs/adapter-vercel
# Update svelte.config.js to use vercel adapter
npm run build
```

### Other Platforms
- **Netlify**: Use `@sveltejs/adapter-netlify`
- **Cloudflare**: Use `@sveltejs/adapter-cloudflare`
- **Node.js**: Use `@sveltejs/adapter-node`

## 🔧 Advanced Configuration

### Custom Subdomain Handling

For production use with custom domains, configure your DNS and load balancer to route subdomains to your SvelteKit application:

```nginx
# Nginx example
server {
    server_name *.yourdomain.com;
    location / {
        proxy_pass http://your-sveltekit-app;
        proxy_set_header Host $host;
    }
}
```

### Webhook Forwarding

The system supports forwarding webhooks to multiple targets with:
- **Automatic retries** for failed requests
- **Custom headers** for identification
- **Error logging** and monitoring

## 📊 Monitoring

- Real-time event count in dashboard
- Connection status indicators
- Event history with filtering
- Relay target success/failure tracking

## 🔮 Future Enhancements

- Webhook filtering and transformation rules
- Custom authentication methods
- Webhook replay functionality
- Analytics and metrics dashboard
- Rate limiting and throttling
- Webhook signature verification for specific providers