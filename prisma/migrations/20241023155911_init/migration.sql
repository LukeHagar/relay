/*
  Warnings:

  - Added the required column `query` to the `WebhookEvent` table without a default value. This is not possible if the table is not empty.

*/
-- AlterTable
ALTER TABLE "WebhookEvent" ADD COLUMN     "query" TEXT NOT NULL;
