//! Declarative, per-node command guards.
//!
//! Pumpkin's WIT contract defines `command-node.require-with-handler-id`,
//! which is the native equivalent of Brigadier's per-node `.requires(...)` —
//! exactly the hook you'd want to declare "needs permission X" / "players
//! only" on a node without touching the handler body. It's not usable yet:
//! the host stub errors unconditionally
//! (`pumpkin-wasm-host-v0_2/src/commands/mod.rs::require_with_handler_id`
//! returns `Err("require_with_handler_id not implemented")`), so calling it
//! would fail at runtime, not just be a no-op.
//!
//! Until that lands upstream, this module gets the same effect from
//! plugin-side Rust: wrap a [`CommandHandler`] once, at the `.execute(...)`
//! call site, instead of copy-pasting the check into every handler's
//! `handle()` body. [`CommandHandlerExt`] and [`PlayerCommandHandlerExt`]
//! turn that composition into a builder-style method chain:
//!
//! ```ignore
//! CommandNode::literal("mv").execute(MvExecutor.player_only().require(PERMISSION))
//! ```
//!
//! New guards follow the same shape: a wrapper struct implementing
//! [`CommandHandler`] by checking something and then delegating to `inner`,
//! plus an extension-trait method that constructs it.

use pumpkin_plugin_api::command::{CommandError, CommandSender, ConsumedArgs};
use pumpkin_plugin_api::commands::CommandHandler;
use pumpkin_plugin_api::player::Player;
use pumpkin_plugin_api::text::TextComponent;
use pumpkin_plugin_api::{Result, Server};

macro_rules! require_permission {
    ($sender:expr, $server:expr, $permission:expr) => {
        if !$sender.has_permission(&$server, $permission) {
            return Err(pumpkin_plugin_api::command::CommandError::CommandFailed(
                pumpkin_plugin_api::text::TextComponent::text("Permission denied."),
            ));
        }
    };
}

/// Like [`CommandHandler`], but receives an already-resolved [`Player`] instead of
/// the raw [`CommandSender`]. Wrap with [`PlayerOnly`] (or `.player_only()`) to use
/// one as a node executor.
pub trait PlayerCommandHandler {
    fn handle(&self, player: Player, server: Server, args: ConsumedArgs) -> Result<i32, CommandError>;
}

/// Rejects non-player senders (console, command blocks), then delegates to `H`
/// with the sender resolved to a [`Player`]. Build with `.player_only()`
/// ([`PlayerCommandHandlerExt`]) rather than constructing directly.
pub struct PlayerOnly<H>(pub H);

impl<H: PlayerCommandHandler + Send + Sync> CommandHandler for PlayerOnly<H> {
    fn handle(&self, sender: CommandSender, server: Server, args: ConsumedArgs) -> Result<i32, CommandError> {
        let Some(player) = sender.as_player() else {
            sender.send_error(TextComponent::text("このコマンドはプレイヤーのみ実行できます。"));
            return Ok(0);
        };
        self.0.handle(player, server, args)
    }
}

/// Requires `permission` on the sender before delegating to `inner`. Build with
/// `.require(permission)` ([`CommandHandlerExt`]) rather than constructing directly.
pub struct RequiresPermission<H> {
    pub permission: &'static str,
    pub inner: H,
}

impl<H: CommandHandler + Send + Sync> CommandHandler for RequiresPermission<H> {
    fn handle(&self, sender: CommandSender, server: Server, args: ConsumedArgs) -> Result<i32, CommandError> {
        require_permission!(sender, server, self.permission);
        self.inner.handle(sender, server, args)
    }
}

/// Builder-style entry point for [`RequiresPermission`]: `XExecutor.require(PERMISSION)`.
pub trait CommandHandlerExt: CommandHandler + Sized {
    fn require(self, permission: &'static str) -> RequiresPermission<Self> {
        RequiresPermission {
            permission,
            inner: self,
        }
    }
}
impl<H: CommandHandler> CommandHandlerExt for H {}

/// Builder-style entry point for [`PlayerOnly`]: `XExecutor.player_only()`.
pub trait PlayerCommandHandlerExt: PlayerCommandHandler + Sized {
    fn player_only(self) -> PlayerOnly<Self> {
        PlayerOnly(self)
    }
}
impl<H: PlayerCommandHandler> PlayerCommandHandlerExt for H {}
