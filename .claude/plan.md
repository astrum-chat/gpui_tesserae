# Fix: UniformListElement uses last width instead of current width

## Problem Analysis

The `UniformListElement` in `elements.rs` has a timing issue where the wrapped line computation uses stale width values, causing premature text wrapping.

### Root Cause

The issue is in the rendering flow:

1. **During `render_wrapped_list`** (in `mod.rs`):
   - `compute_wrap_width()` is called using `cached_wrap_width` from state
   - `precompute_wrapped_lines()` uses this width to compute visual lines
   - The uniform list is then built with these precomputed lines

2. **During `UniformListElement::paint`** (in `elements.rs`):
   - `check_container_width_change()` detects width changes and sets `needs_wrap_recompute = true`
   - `recompute_wrapping_if_needed()` recomputes wrapping if the bounds width differs from `precomputed_at_width`

**The Problem**: The `recompute_wrapping_if_needed()` function uses `bounds.size.width` (the current width) to recompute wrapping. However, during the *initial* render or when width changes, the flow is:

1. Render phase: Uses `cached_wrap_width` (the OLD value or `None`)
2. Paint phase: Gets `bounds` with the NEW width, updates `cached_wrap_width`, sets `needs_wrap_recompute = true`
3. **But the current frame is already rendered with the old width**
4. Next frame: Uses the new `cached_wrap_width` correctly

This one-frame lag causes premature wrapping because:
- On frame N: Text is wrapped using width from frame N-1
- The uniform list's child already rendered with stale wrapping

### The Fix

The issue is that in `render_wrapped_list`, when computing `wrap_width`, we use `cached_wrap_width` which may be stale. We need to use the **current container bounds** when available.

However, looking more carefully at the code flow:
- In `render_wrapped_list`, we don't have access to the actual bounds yet (that comes in paint phase)
- The `cached_wrap_width` is supposed to cache the last known container width to use for wrapping

The real issue is in `recompute_wrapping_if_needed` - it recomputes using `bounds.size.width` but the uniform list's child element was already laid out and prepainted. By the time we detect the width mismatch in paint, it's too late for the current frame.

**Solution**: In `UniformListElement`, we should detect width changes earlier. Since we can't access bounds in `request_layout`, we need to ensure that when `paint` is called:

1. If the width changed significantly, we should use the NEW width immediately for wrapping calculations in this paint pass
2. The `recompute_wrapping_if_needed` already does this, but the child was already painted with old data

The fix is to **call `recompute_wrapping_if_needed` BEFORE painting the child**, so the state is updated before the child reads from it. Currently the order in `paint` is:
1. `check_container_width_change` - updates `cached_wrap_width` and sets `needs_wrap_recompute`
2. `recompute_wrapping_if_needed` - recomputes if needed
3. Paint child

This seems correct, but the issue is that the child element was already prepainted with the old values. The uniform list callback reads from `precomputed_visual_lines` which was populated during render, not during paint.

### Actual Fix

The actual fix needs to happen in the `mod.rs` render phase. When `needs_wrap_recompute` is true AND we have a `cached_wrap_width`, we should use that `cached_wrap_width` as the wrap width instead of computing a potentially stale value.

Looking at `compute_wrap_width`:
```rust
fn compute_wrap_width(
    cached_wrap_width: Option<Pixels>,
    measured_width: Option<Pixels>,
    max_width_px: Option<Pixels>,
    user_wants_auto_width: bool,
) -> Pixels
```

It already prefers `cached_wrap_width` when available. The issue is that `cached_wrap_width` itself lags by one frame because it's set in `paint` but read in `render`.

**The real fix**: In `UniformListElement::prepaint`, we should check if the prepaint bounds differ from `precomputed_at_width` and trigger an immediate recompute there, before the child's prepaint reads the stale data.

Wait, looking at the code again:
- `prepaint` just calls `self.child.prepaint(window, cx)`
- The child is already created during render, with callbacks that capture state

The uniform list callback reads state during the callback execution, which happens during the uniform list's layout/prepaint phase.

**Final Analysis**: The issue is that:
1. `render_wrapped_list` precomputes lines with `wrap_width`
2. The uniform list's render callback reads `precomputed_visual_lines` from state
3. In paint, we detect the width mismatch and update, but it's too late

The fix should be in `UniformListElement::prepaint` - we should detect width changes there (we have access to bounds in prepaint) and recompute wrapping BEFORE the child prepaint runs.

## Implementation Plan

1. Move the width change detection and recomputation from `paint` to `prepaint` in `UniformListElement`
2. In `prepaint`, after we have bounds:
   - Check if `bounds.size.width` differs significantly from `precomputed_at_width`
   - If so, immediately recompute wrapping using the new width
   - Then prepaint the child (which will use the fresh data)
3. Keep the existing paint-phase logic as a fallback

### Code Changes

In `elements.rs`, modify `UniformListElement::prepaint`:

```rust
fn prepaint(
    &mut self,
    _id: Option<&GlobalElementId>,
    _inspector_id: Option<&InspectorElementId>,
    bounds: Bounds<Pixels>,  // Note: bounds IS available here
    _request_layout: &mut Self::RequestLayoutState,
    window: &mut Window,
    cx: &mut App,
) -> Self::PrepaintState {
    // Check and update wrap width BEFORE child prepaint
    self.check_container_width_change(bounds.size.width, cx);
    self.recompute_wrapping_if_needed(bounds, window, cx);

    self.child.prepaint(window, cx);
}
```

And remove the duplicate calls from `paint` (or keep them as fallback).
