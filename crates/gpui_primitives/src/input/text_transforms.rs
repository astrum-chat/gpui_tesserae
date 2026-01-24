/// Replaces any character with '*'. Use with `Input::transform_text` for password fields.
pub fn password(_: char) -> char {
    '*'
}
