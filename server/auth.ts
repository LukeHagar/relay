import type { Serve, Server } from "bun";
import { type ServerWebSocket, type WebSocketHandler } from "bun";
import { createBunWebSocket } from "hono/bun";
import { prisma } from "./db";
import { clients } from ".";
import { Hono } from "hono";
import { HTTPException } from "hono/http-exception";
import { logger } from "hono/logger";
import { authHandler, initAuthConfig, verifyAuth } from "@hono/auth-js";
import { PrismaAdapter } from "@auth/prisma-adapter";
import GitHub from "@auth/core/providers/github";
import type { GitHubProfile } from "@auth/core/providers/github";
import type { Awaitable, User } from "@auth/core/types";

export const AuthConfig = initAuthConfig((c) => ({
  secret: c.env.AUTH_SECRET,
  providers: [
    GitHub({
      clientId: process.env.GITHUB_CLIENT_ID,
      clientSecret: process.env.GITHUB_CLIENT_SECRET,
      profile(profile) {
        return {
          id: profile.id.toString(),
          name: profile.name ?? profile.login,
          username: profile.login,
          email: profile.email,
          image: profile.avatar_url,
          subdomain: crypto.randomUUID(),
        };
      },
    }),
  ],
  adapter: PrismaAdapter(prisma),
}));
