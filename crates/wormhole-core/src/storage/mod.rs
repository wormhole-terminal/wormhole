//! Where Wormhole keeps its data and how it is read and written: the data folder's layout and the
//! profile id that names a folder in it (`paths`), files read with a cap and written atomically
//! (`files`), the hash an approval is bound to (`fingerprint`), JSONC files (`jsonc`) and the SQLite database (`store`).
//!
//! The lowest tier: nothing here uses another domain of core. The store never reads settings or
//! talks to the running app; what it saves comes to it as values.

pub mod files;
pub mod fingerprint;
pub mod jsonc;
pub mod paths;
pub mod store;
