CREATE TABLE AppMeta (Key TEXT PRIMARY KEY NOT NULL, Value TEXT NOT NULL);
CREATE TABLE NativeMigration (Version INTEGER PRIMARY KEY, AppliedAt TEXT NOT NULL, Description TEXT NOT NULL);
ALTER TABLE User ADD COLUMN AuthVersion INTEGER NOT NULL DEFAULT 0;
CREATE TABLE UserSession (
 TokenHash TEXT PRIMARY KEY NOT NULL,
 UserId INTEGER NOT NULL REFERENCES User(Id) ON DELETE CASCADE,
 AuthVersion INTEGER NOT NULL,
 IssuedAt INTEGER NOT NULL,
 ExpiresAt INTEGER NOT NULL,
 LastSeenAt INTEGER NOT NULL,
 Locked INTEGER NOT NULL DEFAULT 0,
 CHECK(ExpiresAt = IssuedAt + 86400)
);
CREATE TABLE UserPreferences (
 UserId INTEGER PRIMARY KEY REFERENCES User(Id) ON DELETE CASCADE,
 TutorialStatus TEXT NOT NULL DEFAULT 'pending' CHECK(TutorialStatus IN ('pending','skipped','in_progress','completed')),
 TutorialStep INTEGER NOT NULL DEFAULT 0,
 Theme TEXT NOT NULL DEFAULT 'system' CHECK(Theme IN ('system','light','dark')),
 ReducedMotion INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE RequestReceipt (RequestId TEXT PRIMARY KEY, Operation TEXT NOT NULL, ResultJson TEXT NOT NULL, CreatedAt TEXT NOT NULL);
CREATE INDEX IX_Session_User ON UserSession(UserId);
INSERT INTO UserPreferences(UserId) SELECT Id FROM User;
INSERT INTO AppMeta VALUES ('product','donovan-pos'),('schema','2');
PRAGMA user_version=2;
