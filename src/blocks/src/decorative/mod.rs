mod banner;
mod bell;
mod candle;
mod chain;
mod chiseled_bookshelf;
mod copper_golem_statue;
mod decorated_pot;
mod door;
mod fence_gate;
mod fence_pane;
mod hanging_sign;
mod lantern;
mod note_block;
mod shelf;
mod sign;
mod trapdoor;
mod wall;
mod wall_hanging_sign;
mod wall_sign;
mod waterloggable_wall;

/// Standing signs use a 16-step rotation rather than a 4-direction facing.
/// Rotation 0 is south, increasing counter-clockwise in 22.5 degree steps —
/// the same convention as `Direction::from_yaw`, but the value is where the
/// sign's face points, so it's the player's yaw turned around.
pub(crate) fn yaw_to_sign_rotation(yaw: f32) -> i32 {
    ((yaw / 22.5 + 0.5).floor() as i32 + 8).rem_euclid(16)
}
