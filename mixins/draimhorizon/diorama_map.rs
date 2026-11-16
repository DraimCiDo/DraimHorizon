// The Diorama map: the 3D alternative to the flat map. While "Map view: 3D" is on, this module
// makes the minimap and the M map's picture (from the loaded chunks, by the Diorama core in
// `diorama_view`) and the flat map's scanner, worker and history do not run at all; with 3D off
// the flat map runs as it always did. The two share the screens (buttons, marks, mouse), not the
// map logic.
use super::*;
use crate::game::{claim_borders_map as model, claim_borders_map_view as view};
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

/// The minimap and the world map's picture are made this often.
const REFRESH: Duration = Duration::from_millis(100);
const WORLD_SPRITE: &str = "claim_borders/world_map";
/// Pixels per side of the world map's picture.
const FULL_SIZE: usize = 512;

struct Clock {
    minimap: Option<Instant>,
    full: Option<(Instant, model::View, u32)>,
}

static CLOCK: Mutex<Clock> = Mutex::new(Clock {
    minimap: None,
    full: None,
});

/// One frame of the 3D map; returns what the flat `update` returns (whether the map is on).
pub(in crate::game) fn update(game: &mut Game<'_>, request: &view::Request) -> bool {
    let p = request.preferences;
    crate::game::claim_borders_diorama_view::configure(p.pitch3d, p.shadow3d, p.sharp3d);
    if !request.draw {
        return true;
    }
    let scope = format!("{}|{}", request.source.scope, request.dimension);
    // The angled map is of the world the player is in: only its chunks are loaded.
    let here = request.dimension == request.source.dimension;
    let mut clock = CLOCK.lock().unwrap_or_else(|e| e.into_inner());
    if !request.full {
        if clock.minimap.is_some_and(|t| t.elapsed() < REFRESH)
            && !game.extras.minimap.image.is_empty()
        {
            return true;
        }
        clock.minimap = Some(Instant::now());
        let Some(rect) = game.gui.atlas.sprites.get(MINIMAP_SPRITE).copied() else {
            return true;
        };
        let size = minimap::SIZE;
        let Some(pixels) = here
            .then(|| {
                crate::game::claim_borders_diorama_view::minimap(
                    game,
                    &scope,
                    request.view.center,
                    request.source.player[1],
                    request.view.blocks_per_pixel,
                    size,
                )
            })
            .flatten()
        else {
            return true;
        };
        let (w, h) = (game.gui.atlas.width as f32, game.gui.atlas.height as f32);
        game.map_patches.push(horizon_client::font::Patch {
            x: (rect.u0 * w).round() as u32,
            y: (rect.v0 * h).round() as u32,
            width: size as u32,
            height: size as u32,
            rgba: pixels,
        });
        if game.extras.minimap.image.is_empty() {
            game.extras.minimap.image = vec![0; size * size * 4];
        }
        view::mini_published(request.dimension.clone(), request.view);
        return true;
    }
    let moved = clock.full.is_none_or(|(t, old, aspect)| {
        t.elapsed() >= REFRESH || old != request.view || aspect != request.aspect.to_bits()
    });
    if !moved || !here {
        return true;
    }
    clock.full = Some((Instant::now(), request.view, request.aspect.to_bits()));
    let Some(rect) = game.gui.atlas.sprites.get(WORLD_SPRITE).copied() else {
        return true;
    };
    let Some((pixels, bpp)) = crate::game::claim_borders_diorama_view::full_image(
        game,
        &scope,
        request.view.center,
        request.source.player[1],
        f64::from(request.view.blocks_per_pixel),
        f64::from(request.aspect),
        FULL_SIZE,
    ) else {
        return true;
    };
    let (w, h) = (game.gui.atlas.width as f32, game.gui.atlas.height as f32);
    game.map_patches.push(horizon_client::font::Patch {
        x: (rect.u0 * w).round() as u32,
        y: (rect.v0 * h).round() as u32,
        width: FULL_SIZE as u32,
        height: FULL_SIZE as u32,
        rgba: pixels,
    });
    view::published(
        request.dimension.clone(),
        model::View {
            center: request.view.center,
            blocks_per_pixel: bpp as f32,
        },
        request.aspect,
    );
    true
}
