// Diorama angled map, game side: builds the block looks from the resource pack on a worker, feeds
// loaded chunks to the Diorama core's mosaic within a time budget, and cuts the minimap window out
// of it. The flat map stays as it was; this runs only while "Map view: 3D" is on.
use super::*;
use crate::game::{
    diorama_bake as bake, diorama_looks as looks, diorama_mosaic as mosaic, diorama_paint as paint,
    diorama_projection::Projection, diorama_style as style,
};
use std::{
    collections::BTreeMap,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

/// Blocks read down from each column's top: south faces and water floors need them.
const DEPTH: u8 = 16;
/// Texels per side of the mosaic (384 blocks at four texels a block).
const SIDE: u32 = 1536;
const TEXELS: u32 = 4;
/// Samples per output pixel along each axis.
const SUPER: usize = 2;
/// Game-thread time spent reading chunks and painting rows per call.
const READ_BUDGET: Duration = Duration::from_micros(1200);
const PAINT_BUDGET: Duration = Duration::from_micros(1500);
/// Chunks already read near the player are read again this often, so edits show.
const REFRESH: Duration = Duration::from_secs(3);
/// The nearest chunks are refreshed faster.
const NEAR_REFRESH: Duration = Duration::from_millis(800);
/// The widest view (blocks per pixel) the mosaic can hold across the 128 pixel box.
pub(super) const MAX_BPP: f32 = 2.5;

struct State {
    generation: u64,
    task: Option<(u64, std::thread::JoinHandle<Option<paint::Painter>>)>,
    painter: Option<paint::Painter>,
    mosaic: mosaic::Mosaic,
    scope: String,
    /// When each chunk was read and a fingerprint of what was read.
    fed: BTreeMap<(i32, i32), (Instant, u64)>,
    /// The top heights of every chunk read, for sun shadows and corner occlusion across chunk edges.
    heights: BTreeMap<(i32, i32), Vec<i16>>,
    /// Chunks read before all their neighbours were known: read again once they are.
    partial: std::collections::BTreeSet<(i32, i32)>,
}

fn new_mosaic() -> mosaic::Mosaic {
    mosaic::Mosaic::with_texels(Projection::new(style::DEFAULT.pitch_degrees), SIDE, TEXELS)
}

fn state<T>(f: impl FnOnce(&mut State) -> T) -> T {
    static STATE: OnceLock<Mutex<State>> = OnceLock::new();
    let mut s = STATE
        .get_or_init(|| {
            Mutex::new(State {
                generation: 0,
                task: None,
                painter: None,
                mosaic: new_mosaic(),
                scope: String::new(),
                fed: BTreeMap::new(),
                heights: BTreeMap::new(),
                partial: Default::default(),
            })
        })
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    f(&mut s)
}

static SHOWING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Whether the last minimap frame was the angled view (marks then use its projection).
pub(super) fn showing() -> bool {
    SHOWING.load(std::sync::atomic::Ordering::Relaxed)
}

/// Where a world point lands in the angled minimap's 128 pixel space, for a mark at the player's
/// height: east is right, south is down by `sin(pitch)`.
pub(super) fn mark_point(
    view_centre: [f64; 2],
    blocks_per_pixel: f32,
    world: [f64; 2],
) -> [f32; 2] {
    let bpp = f64::from(blocks_per_pixel.clamp(0.25, MAX_BPP));
    let sin = style::DEFAULT.pitch_degrees.to_radians().sin();
    [
        (64.0 + (world[0] - view_centre[0]) / bpp) as f32,
        (64.0 + (world[1] - view_centre[1]) * sin / bpp) as f32,
    ]
}

/// Drops the angled picture's state the first time the flat view is shown again.
pub(super) fn reset_if_showing() {
    if SHOWING.swap(false, std::sync::atomic::Ordering::Relaxed) {
        reset();
    }
}

/// Forgets everything painted (leaving the world, or the 3D view being switched off).
pub(super) fn reset() {
    SHOWING.store(false, std::sync::atomic::Ordering::Relaxed);
    state(|s| {
        s.scope.clear();
        s.fed.clear();
        s.heights.clear();
        s.partial.clear();
        s.mosaic.clear();
    });
}

/// The Diorama core's block table from the game's states and the loaded resource pack.
fn table(
    states: &horizon_client::block_states::BlockStates,
    render: &horizon_client::assets::RenderAssets,
) -> looks::Table {
    use horizon_client::mesher::TintSource;
    let n = states.len();
    let mut sprites = vec![String::new(); render.texture_layers as usize];
    for (name, layer) in &render.sprite_layers {
        if let Some(slot) = sprites.get_mut(*layer as usize) {
            *slot = name.clone();
        }
    }
    let layer_of = |sprite: horizon_client::block_sprites::SpriteRef| -> Option<u32> {
        let name = states.texture(sprite)?;
        render
            .sprite_layers
            .get(name)
            .or_else(|| render.sprite_layers.get(&format!("minecraft:{name}")))
            .copied()
    };
    let none = looks::Face {
        sprite: looks::NO_SPRITE,
        tint: looks::Tint::None,
    };
    let mut table = looks::Table {
        version: render.generation,
        sprites,
        ..looks::Table::default()
    };
    let mut block_ids = BTreeMap::<String, u16>::new();
    for (id, facts) in states.iter() {
        let name = facts.name.to_string();
        let next = block_ids.len() as u16;
        let block = *block_ids.entry(name.clone()).or_insert_with(|| {
            table.blocks.push(name.clone());
            next
        });
        let mut f = 0u16;
        if facts.air {
            f |= looks::flags::AIR;
        }
        if facts.invisible {
            f |= looks::flags::INVISIBLE;
        }
        match name.as_str() {
            "minecraft:water"
            | "minecraft:bubble_column"
            | "minecraft:kelp"
            | "minecraft:kelp_plant"
            | "minecraft:seagrass"
            | "minecraft:tall_seagrass" => f |= looks::flags::WATER,
            "minecraft:lava" => f |= looks::flags::LAVA,
            _ => {}
        }
        if facts.waterlogged {
            f |= looks::flags::WATERLOGGED;
        }
        if !facts.collides {
            f |= looks::flags::NO_COLLISION;
        }
        if facts.outline.len() == 1 && facts.outline[0] == [0.0, 0.0, 0.0, 1.0, 1.0, 1.0] {
            f |= looks::flags::FULL_TOP;
        }
        f |= u16::from(facts.emission.min(15)) << looks::flags::EMISSION_SHIFT;
        let mut constant = 0;
        let mut face = |sides: &[horizon_client::block_sprites::FaceSprite]| {
            // A grass block's side is its dirt, then the tinted fringe over it: the tinted one wins.
            let Some(face) = sides.iter().find(|s| s.tint.is_some()).or(sides.first()) else {
                return none;
            };
            let Some(sprite) = layer_of(face.sprite) else {
                return none;
            };
            let tint = match face.tint {
                None => looks::Tint::None,
                Some(TintSource::Grass) => looks::Tint::Grass,
                Some(TintSource::Foliage) => looks::Tint::Foliage,
                Some(TintSource::DryFoliage) => looks::Tint::DryFoliage,
                Some(TintSource::Water) => looks::Tint::Water,
                Some(TintSource::Fixed(color)) => {
                    constant = color & 0xFF_FFFF;
                    looks::Tint::Constant
                }
            };
            looks::Face { sprite, tint }
        };
        let (top, side) = match states.sprites(id) {
            // Direction::ALL order: down, up, north, south, west, east.
            Some(s) => (face(&s.sides[1]), face(&s.sides[3])),
            None => (none, none),
        };
        table.state_block.push(block);
        table.state_flags.push(f);
        table.state_top.push(top);
        table.state_side.push(side);
        table.constant_tint.push(constant);
        table.map_color.push(facts.map_color.unwrap_or(0));
    }
    debug_assert_eq!(table.state_flags.len(), n);
    table
}

/// Every used texture's average, then the looks and a painter in Diorama's default style.
fn build(
    states: &horizon_client::block_states::BlockStates,
    render: &horizon_client::assets::RenderAssets,
) -> paint::Painter {
    let table = table(states, render);
    let pixels = render.pixels.get();
    let side = render.sprite_size as usize;
    let stride = side * side * 4;
    let mut averages = vec![None; table.sprites.len()];
    if stride > 0 {
        for sprite in table.used_sprites() {
            let at = sprite as usize * stride;
            if let Some(layer) = pixels.get(at..at + stride) {
                averages[sprite as usize] = Some(looks::average(side as u32, side as u32, layer));
            }
        }
    }
    paint::Painter::new(looks::Looks::build(&table, &averages), style::DEFAULT)
}

/// One chunk's columns in the core's format, or none for a chunk with nothing in it.
fn read_chunk(
    game: &Game<'_>,
    painter: &paint::Painter,
    at: (i32, i32),
    chunk: &horizon_client::chunk::Chunk,
) -> bake::Chunk {
    let biomes = world_biomes!(game);
    let d = usize::from(DEPTH);
    let mut tops = vec![bake::NO_TOP; 256];
    let mut states = vec![0u32; 256 * d];
    let mut water = vec![0u8; 256];
    let mut tints = vec![0xFF_FFFFu32; 256 * 3];
    let min_y = chunk.min_y();
    for z in 0..16i32 {
        for x in 0..16i32 {
            let i = (z * 16 + x) as usize;
            let kind = |y: i32| painter.looks.get(chunk.get(x, y, z)).kind;
            let Some(top) = chunk.highest(0, |y| kind(y) != looks::Kind::Air) else {
                continue;
            };
            tops[i] = top as i16;
            for k in 0..d {
                let y = top - k as i32;
                states[i * d + k] = if y >= min_y { chunk.get(x, y, z) } else { 0 };
            }
            if kind(top) == looks::Kind::Water {
                let mut depth = 0u8;
                let mut y = top;
                while y >= min_y && kind(y) == looks::Kind::Water && depth < 255 {
                    depth += 1;
                    y -= 1;
                }
                water[i] = depth;
            }
            let biome = chunk.biome(x, top, z);
            let b = biomes.get(biome);
            let (wx, wz) = (f64::from(at.0 * 16 + x), f64::from(at.1 * 16 + z));
            tints[i * 3] = biomes.grass(biome, wx, wz) & 0xFF_FFFF;
            tints[i * 3 + 1] = b.foliage & 0xFF_FFFF;
            tints[i * 3 + 2] = b.water & 0xFF_FFFF;
        }
    }
    let mut out = bake::Chunk::new(tops, states, DEPTH);
    out.water = water;
    out.tints = tints;
    painter.prepare(&mut out);
    out
}

/// The minimap's pixels (`size` x `size`, RGBA) in the angled view around `centre` (block x, z) at
/// the player's height `y`, or none while the block looks are still being built.
pub(super) fn minimap(
    game: &Game<'_>,
    scope: &str,
    centre: [f64; 2],
    y: f64,
    blocks_per_pixel: f32,
    size: usize,
) -> Option<Vec<u8>> {
    let bpp = f64::from(blocks_per_pixel.clamp(0.25, MAX_BPP));
    let generation = game.pack.render.generation;
    state(|s| {
        if s.generation != generation {
            s.generation = generation;
            s.painter = None;
            s.task = None;
            s.fed.clear();
            s.heights.clear();
            s.partial.clear();
            s.mosaic.clear();
        }
        if s.task.as_ref().is_some_and(|(_, t)| t.is_finished()) {
            let (at, task) = s.task.take().unwrap();
            if at == generation
                && let Ok(Some(painter)) = task.join()
            {
                s.painter = Some(painter);
                s.fed.clear();
                s.heights.clear();
                s.partial.clear();
                s.mosaic.clear();
            }
        }
        if s.painter.is_none() {
            if s.task.is_none() {
                let states = game.native_block_states();
                let render = game.pack.render.clone();
                if let Ok(task) = std::thread::Builder::new()
                    .name("diorama-looks".into())
                    .spawn(move || Some(build(&states, &render)))
                {
                    s.task = Some((generation, task));
                }
            }
            SHOWING.store(false, std::sync::atomic::Ordering::Relaxed);
            return None;
        }
        if s.scope != scope {
            s.scope = scope.to_owned();
            s.fed.clear();
            s.heights.clear();
            s.partial.clear();
            s.mosaic.clear();
        }
        let painter = s.painter.take().unwrap();
        let proj = *s.mosaic.projection();
        let (cu, cv) = (centre[0], proj.v(y, centre[1]));
        s.mosaic.recentre(cu, cv);
        let half = bpp * size as f64 / 2.0;
        let (t0u, t0v) = s.mosaic.texel_of(cu - half, cv - half);
        let (t1u, t1v) = s.mosaic.texel_of(cu + half, cv + half);
        let clamp = |t: f64| t.clamp(0.0, f64::from(SIDE)) as u32;
        s.mosaic
            .set_focus(Some((clamp(t0u), clamp(t0v), clamp(t1u), clamp(t1v))));

        // Read chunks, the nearest first, within the time budget.
        let (pcx, pcz) = (
            (centre[0].floor() as i32) >> 4,
            (centre[1].floor() as i32) >> 4,
        );
        let radius = ((half / 16.0).ceil() as i32 + 3).min(14);
        let mut wanted: Vec<(i32, i32)> = (-radius..=radius)
            .flat_map(|dz| (-radius..=radius).map(move |dx| (pcx + dx, pcz + dz)))
            .filter(|at| game.world.chunks.contains_key(at))
            .collect();
        wanted.sort_by_key(|&(x, z)| (x - pcx).abs().max((z - pcz).abs()));
        let started = Instant::now();
        for at in wanted {
            let near = (at.0 - pcx).abs().max((at.1 - pcz).abs()) <= 2;
            let healed = s.partial.contains(&at)
                && s.heights.contains_key(&(at.0 + 1, at.1))
                && s.heights.contains_key(&(at.0 - 1, at.1))
                && s.heights.contains_key(&(at.0, at.1 + 1))
                && s.heights.contains_key(&(at.0, at.1 - 1));
            let due = match s.fed.get(&at) {
                None => true,
                Some(_) if healed => true,
                Some((t, _)) => t.elapsed() >= if near { NEAR_REFRESH } else { REFRESH },
            };
            // Far chunks are read once; only the neighbourhood is read again.
            if !due || (s.fed.contains_key(&at) && !near && !healed) {
                continue;
            }
            if started.elapsed() >= READ_BUDGET {
                break;
            }
            let Some(chunk) = game.world.chunks.get(&at) else {
                continue;
            };
            let mut read = read_chunk(game, &painter, at, chunk);
            s.heights.insert(at, read.tops.clone());
            let complete = shade(&mut read, at, &s.heights);
            if complete {
                s.partial.remove(&at);
            } else {
                s.partial.insert(at);
            }
            let print = fingerprint(&read);
            if s.fed.get(&at).is_none_or(|(_, old)| *old != print) {
                s.mosaic.set_chunk(at, Some(read));
            }
            s.fed.insert(at, (Instant::now(), print));
        }
        s.mosaic.forget_far((pcx, pcz), radius + 4);
        s.heights
            .retain(|&(x, z), _| (x - pcx).abs() <= radius + 5 && (z - pcz).abs() <= radius + 5);
        s.partial.retain(|k| s.fed.contains_key(k));
        s.fed
            .retain(|&(x, z), _| (x - pcx).abs() <= radius + 4 && (z - pcz).abs() <= radius + 4);

        let painted = Instant::now();
        while painted.elapsed() < PAINT_BUDGET && s.mosaic.paint_step(&painter, cu, 24) {}

        let outline = painter.style.outline;
        let mut out = Vec::with_capacity(size * size * 4);
        let limit = f64::from(SIDE) - 1.0;
        for py in 0..size {
            for px in 0..size {
                // 2 x 2 samples a pixel: the host's picture is only 128 pixels wide, so the detail
                // the mosaic holds is averaged down instead of picked at one texel.
                let (mut sum, mut seen) = ([0u32; 3], 0u32);
                for sy in 0..SUPER {
                    for sx in 0..SUPER {
                        let u = cu
                            + (px as f64 + (sx as f64 + 0.5) / SUPER as f64 - size as f64 / 2.0)
                                * bpp;
                        let v = cv
                            + (py as f64 + (sy as f64 + 0.5) / SUPER as f64 - size as f64 / 2.0)
                                * bpp;
                        let (ti, tj) = s.mosaic.texel_of(u, v);
                        if (0.0..=limit).contains(&ti) && (0.0..=limit).contains(&tj) {
                            let [r, g, b, a] = s.mosaic.shown_at(ti as u32, tj as u32, outline);
                            if a != 0 {
                                sum[0] += u32::from(r);
                                sum[1] += u32::from(g);
                                sum[2] += u32::from(b);
                                seen += 1;
                            }
                        }
                    }
                }
                out.extend(if seen == 0 {
                    let shade = if (px / 16 + py / 16) % 2 == 0 { 20 } else { 23 };
                    [shade, shade + 5, shade + 10, 255]
                } else {
                    [
                        (sum[0] / seen) as u8,
                        (sum[1] / seen) as u8,
                        (sum[2] / seen) as u8,
                        255,
                    ]
                });
            }
        }
        s.painter = Some(painter);
        SHOWING.store(true, std::sync::atomic::Ordering::Relaxed);
        Some(out)
    })
}

/// A cheap fingerprint of a read chunk, so an unchanged one is not painted again.
fn fingerprint(chunk: &bake::Chunk) -> u64 {
    let mut h = 0xcbf29ce484222325u64;
    let mut mix = |v: u64| h = (h ^ v).wrapping_mul(0x100000001b3);
    chunk.tops.iter().for_each(|&t| mix(t as u16 as u64));
    chunk.states.iter().for_each(|&t| mix(u64::from(t)));
    chunk.water.iter().for_each(|&t| mix(u64::from(t)));
    chunk.sun.iter().for_each(|&t| mix(u64::from(t)));
    chunk.ao.iter().for_each(|&t| mix(u64::from(t)));
    h
}

/// Sun shadow and corner occlusion for every column of `chunk`, from the tops of it and its
/// neighbours (blocks away from the known chunks count as level ground). Returns whether all four
/// edge neighbours were known.
fn shade(
    chunk: &mut bake::Chunk,
    at: (i32, i32),
    heights: &BTreeMap<(i32, i32), Vec<i16>>,
) -> bool {
    let (sx, sy, sz) = style::DEFAULT.sun();
    let horizontal = (sx * sx + sz * sz).sqrt();
    // Blocks the sun ray climbs per block walked toward it.
    let slope = sy / horizontal;
    let (dx, dz) = (sx / horizontal, sz / horizontal);
    let height = |x: i32, z: i32| -> Option<i32> {
        let key = (x.div_euclid(16), z.div_euclid(16));
        let tops = heights.get(&key)?;
        let t = tops[(z.rem_euclid(16) * 16 + x.rem_euclid(16)) as usize];
        (t != bake::NO_TOP).then_some(i32::from(t))
    };
    let mut sun = vec![255u8; 256];
    let mut ao = vec![255u8; 256];
    for lz in 0..16i32 {
        for lx in 0..16i32 {
            let i = (lz * 16 + lx) as usize;
            let (x, z) = (at.0 * 16 + lx, at.1 * 16 + lz);
            let Some(h) = height(x, z) else { continue };
            let top = f64::from(h) + 1.0;
            // Sun: walk toward it; a block whose top is above the ray shades this one. The shade
            // softens with how far above and how far away the blocker is.
            let mut lit = 1.0f64;
            for step in 1..=28 {
                let d = f64::from(step);
                let (wx, wz) = (
                    (f64::from(x) + 0.5 + dx * d).floor() as i32,
                    (f64::from(z) + 0.5 + dz * d).floor() as i32,
                );
                let Some(n) = height(wx, wz) else { continue };
                let over = f64::from(n) + 1.0 - (top + slope * d);
                if over > 0.0 {
                    lit = lit.min(1.0 - (over / 1.5).min(1.0));
                    if lit <= 0.0 {
                        break;
                    }
                }
            }
            sun[i] = (lit * 255.0).round() as u8;
            // Corner occlusion: the eight neighbours standing above this block close it in.
            let mut closed = 0.0f64;
            for (ox, oz) in [
                (-1, -1),
                (0, -1),
                (1, -1),
                (-1, 0),
                (1, 0),
                (-1, 1),
                (0, 1),
                (1, 1),
            ] {
                let n = height(x + ox, z + oz).unwrap_or(h);
                let rise = f64::from(n - h).clamp(0.0, 3.0) / 3.0;
                closed += rise * if ox != 0 && oz != 0 { 0.7 } else { 1.0 };
            }
            let open = 1.0 - 0.5 * (closed / 6.0).min(1.0);
            ao[i] = (open * 255.0).round() as u8;
        }
    }
    chunk.sun = sun;
    chunk.ao = ao;
    [(1, 0), (-1, 0), (0, 1), (0, -1)]
        .iter()
        .all(|&(a, b)| heights.contains_key(&(at.0 + a, at.1 + b)))
}
