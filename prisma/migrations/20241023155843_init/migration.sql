/*
  Warnings:

  - Added the required column `path` to the `WebhookEvent` table without a default value. This is not possible if the table is not empty.

*/
-- AlterTable
ALTER TABLE "WebhookEvent" ADD COLUMN     "path" TEXT NOT NULL;
