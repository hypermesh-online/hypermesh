// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! InboxStore — persistent storage for received share invitations.
//!
//! Thread-safe (via `tokio::sync::RwLock`) and optionally backed by a
//! JSON file on disk for persistence across restarts.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;

use super::invite::ShareInvite;

/// In-memory + optional disk-backed store for received share invitations.
pub struct InboxStore {
    invites: Arc<RwLock<HashMap<String, ShareInvite>>>,
    data_dir: Option<PathBuf>,
}

impl InboxStore {
    /// Create a new empty inbox.
    ///
    /// If `data_dir` is `Some`, [`persist`] and [`load`] will read/write
    /// `{data_dir}/inbox.json`.
    pub fn new(data_dir: Option<PathBuf>) -> Self {
        Self {
            invites: Arc::new(RwLock::new(HashMap::new())),
            data_dir,
        }
    }

    /// Add (or replace) an invite in the inbox.
    pub async fn add(&self, invite: ShareInvite) -> anyhow::Result<()> {
        let id = invite.invite_id.clone();
        self.invites.write().await.insert(id, invite);
        Ok(())
    }

    /// List all invites, sorted by `created_at` descending (newest first).
    pub async fn list(&self) -> Vec<ShareInvite> {
        let map = self.invites.read().await;
        let mut items: Vec<ShareInvite> = map.values().cloned().collect();
        items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        items
    }

    /// Get a single invite by ID.
    pub async fn get(&self, invite_id: &str) -> Option<ShareInvite> {
        self.invites.read().await.get(invite_id).cloned()
    }

    /// Remove and return an invite by ID.
    pub async fn remove(&self, invite_id: &str) -> Option<ShareInvite> {
        self.invites.write().await.remove(invite_id)
    }

    /// Number of invites in the inbox.
    pub async fn count(&self) -> usize {
        self.invites.read().await.len()
    }

    /// Persist the current inbox to `{data_dir}/inbox.json`.
    ///
    /// No-op if `data_dir` was not provided.
    pub async fn persist(&self) -> anyhow::Result<()> {
        let dir = match &self.data_dir {
            Some(d) => d,
            None => return Ok(()),
        };
        std::fs::create_dir_all(dir)
            .map_err(|e| anyhow::anyhow!("Failed to create inbox dir: {e}"))?;
        let path = dir.join("inbox.json");
        let map = self.invites.read().await;
        let items: Vec<&ShareInvite> = map.values().collect();
        let json = serde_json::to_vec_pretty(&items)
            .map_err(|e| anyhow::anyhow!("Failed to serialize inbox: {e}"))?;
        std::fs::write(&path, &json)
            .map_err(|e| anyhow::anyhow!("Failed to write inbox file: {e}"))?;
        Ok(())
    }

    /// Load invites from `{data_dir}/inbox.json`, merging into current state.
    ///
    /// No-op if `data_dir` was not provided or the file does not exist.
    pub async fn load(&self) -> anyhow::Result<()> {
        let dir = match &self.data_dir {
            Some(d) => d,
            None => return Ok(()),
        };
        let path = dir.join("inbox.json");
        if !path.exists() {
            return Ok(());
        }
        let data = std::fs::read(&path)
            .map_err(|e| anyhow::anyhow!("Failed to read inbox file: {e}"))?;
        let items: Vec<ShareInvite> = serde_json::from_slice(&data)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize inbox: {e}"))?;
        let mut map = self.invites.write().await;
        for invite in items {
            map.insert(invite.invite_id.clone(), invite);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_invite(id: &str, created_at: i64) -> ShareInvite {
        ShareInvite::new(
            id.into(),
            "asset-1".into(),
            "sender".into(),
            None,
            "recipient".into(),
            "file.bin".into(),
            1024,
            4,
            b"[]".to_vec(),
            vec![0xAA; 32],
            vec![0xBB; 64],
            created_at,
        )
    }

    #[tokio::test]
    async fn test_inbox_add_list_remove() {
        let store = InboxStore::new(None);

        store.add(make_invite("a", 100)).await.expect("test: add a");
        store.add(make_invite("b", 200)).await.expect("test: add b");

        let list = store.list().await;
        assert_eq!(list.len(), 2);
        // Newest first
        assert_eq!(list[0].invite_id, "b");
        assert_eq!(list[1].invite_id, "a");

        let got = store.get("a").await;
        assert!(got.is_some());
        assert_eq!(got.expect("test: get a").invite_id, "a");

        let removed = store.remove("a").await;
        assert!(removed.is_some());
        assert_eq!(store.count().await, 1);

        assert!(store.get("a").await.is_none());
    }

    #[tokio::test]
    async fn test_inbox_count() {
        let store = InboxStore::new(None);
        assert_eq!(store.count().await, 0);

        store.add(make_invite("x", 10)).await.expect("test: add");
        assert_eq!(store.count().await, 1);

        store.add(make_invite("y", 20)).await.expect("test: add");
        assert_eq!(store.count().await, 2);

        store.remove("x").await;
        assert_eq!(store.count().await, 1);
    }

    #[tokio::test]
    async fn test_inbox_persistence() {
        let dir = tempfile::tempdir().expect("test: tempdir");
        let inbox_dir = dir.path().join("inbox");

        // Create and persist
        let store1 = InboxStore::new(Some(inbox_dir.clone()));
        store1
            .add(make_invite("p1", 300))
            .await
            .expect("test: add");
        store1
            .add(make_invite("p2", 400))
            .await
            .expect("test: add");
        store1.persist().await.expect("test: persist");

        // Load into a new store
        let store2 = InboxStore::new(Some(inbox_dir));
        store2.load().await.expect("test: load");

        assert_eq!(store2.count().await, 2);
        assert!(store2.get("p1").await.is_some());
        assert!(store2.get("p2").await.is_some());
    }

    #[tokio::test]
    async fn test_inbox_load_nonexistent_is_noop() {
        let dir = tempfile::tempdir().expect("test: tempdir");
        let store = InboxStore::new(Some(dir.path().join("nonexistent")));
        // Should succeed silently
        store.load().await.expect("test: load nonexistent");
        assert_eq!(store.count().await, 0);
    }
}
