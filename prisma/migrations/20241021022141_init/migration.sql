/*
  Warnings:

  - You are about to drop the column `xForwardedFor` on the `WebsocketEvent` table. All the data in the column will be lost.

*/
-- RedefineTables
PRAGMA defer_foreign_keys=ON;
PRAGMA foreign_keys=OFF;
CREATE TABLE "new_WebsocketEvent" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "event" TEXT NOT NULL,
    "userId" TEXT NOT NULL,
    "createdAt" DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "WebsocketEvent_userId_fkey" FOREIGN KEY ("userId") REFERENCES "User" ("id") ON DELETE RESTRICT ON UPDATE CASCADE
);
INSERT INTO "new_WebsocketEvent" ("createdAt", "event", "id", "userId") SELECT "createdAt", "event", "id", "userId" FROM "WebsocketEvent";
DROP TABLE "WebsocketEvent";
ALTER TABLE "new_WebsocketEvent" RENAME TO "WebsocketEvent";
PRAGMA foreign_keys=ON;
PRAGMA defer_foreign_keys=OFF;
