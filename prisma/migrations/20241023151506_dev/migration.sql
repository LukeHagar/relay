/*
  Warnings:

  - You are about to drop the column `uniqueKey` on the `User` table. All the data in the column will be lost.

*/
-- DropIndex
DROP INDEX "User_uniqueKey_key";

-- AlterTable
ALTER TABLE "User" DROP COLUMN "uniqueKey";
