//! The main program!

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(unused_mut)]

extern crate dwarf_term;
pub use dwarf_term::*;

extern crate roguelike_tutorial_2018;
use roguelike_tutorial_2018::*;

extern crate serde;

extern crate bincode;

// std
use std::collections::btree_map::*;
use std::collections::hash_set::*;
use std::io::*;

const TILE_GRID_WIDTH: usize = 66;
const TILE_GRID_HEIGHT: usize = 50;
const KINDA_LIME_GREEN: u32 = rgb32!(128, 255, 20);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DisplayMode {
  Game,
  Inventory,
  ItemTargeting(char, Location),
}

fn main() {
  let mut term = unsafe { DwarfTerm::new(TILE_GRID_WIDTH, TILE_GRID_HEIGHT, "Kasidin").expect("WHOOPS!") };
  term.set_all_foregrounds(KINDA_LIME_GREEN);
  term.set_all_backgrounds(0);

  let mut game = GameWorld::new(u64_from_time());

  // Main loop
  let mut running = true;
  let mut pending_keys = vec![];
  let mut display_mode = DisplayMode::Game;
  let mut seen_set = HashSet::new();
  'game: loop {
    // Grab all new presses
    term.poll_events(|event| match event {
      Event::WindowEvent { event: win_event, .. } => match win_event {
        WindowEvent::CloseRequested => {
          running = false;
        }
        WindowEvent::KeyboardInput {
          input:
            KeyboardInput {
              state: ElementState::Pressed,
              virtual_keycode: Some(key),
              modifiers: mods,
              ..
            },
          ..
        } => {
          pending_keys.push((key, mods.shift));
        }
        _ => {}
      },
      _ => {}
    });
    if !running {
      break 'game;
    }

    for key in pending_keys.drain(..) {
      match display_mode {
        DisplayMode::Game => match key {
          (VirtualKeyCode::Up, false) => game.move_player(Location { x: 0, y: 1, z: 0 }),
          (VirtualKeyCode::Down, false) => game.move_player(Location { x: 0, y: -1, z: 0 }),
          (VirtualKeyCode::Left, false) => game.move_player(Location { x: -1, y: 0, z: 0 }),
          (VirtualKeyCode::Right, false) => game.move_player(Location { x: 1, y: 0, z: 0 }),
          (VirtualKeyCode::I, false) => display_mode = DisplayMode::Inventory,
          (VirtualKeyCode::F5, false) => {
            save_game(&game).ok();
          }
          (VirtualKeyCode::F6, false) => {
            load_game(&mut game).ok();
          }
          (VirtualKeyCode::Period, true) => game.change_floor(-1),
          (VirtualKeyCode::Comma, true) => game.change_floor(1),
          _ => {}
        },
        DisplayMode::Inventory => match key {
          (VirtualKeyCode::Escape, false) => display_mode = DisplayMode::Game,
          (other, shift) => {
            letter_of(other).map(|ch| {
              if ch.is_alphabetic() && !shift {
                match game.use_item(ch) {
                  UseItemResult::NoSuchItem => {}
                  UseItemResult::ItemUsed => {
                    display_mode = DisplayMode::Game;
                  }
                  UseItemResult::ItemNeedsTarget => {
                    display_mode = DisplayMode::ItemTargeting(ch, Location { x: 0, y: 0, z: 0 });
                  }
                }
              }
            });
          }
        },
        DisplayMode::ItemTargeting(letter, delta) => match key {
          (VirtualKeyCode::Escape, false) => display_mode = DisplayMode::Game,
          (VirtualKeyCode::Return, false) => {
            game.use_targeted_item(letter, delta);
            display_mode = DisplayMode::Game;
          }
          (VirtualKeyCode::Up, false) | (VirtualKeyCode::Down, false) | (VirtualKeyCode::Left, false) | (VirtualKeyCode::Right, false) => {
            let delta_change = match key.0 {
              VirtualKeyCode::Up => Location { x: 0, y: 1, z: 0 },
              VirtualKeyCode::Down => Location { x: 0, y: -1, z: 0 },
              VirtualKeyCode::Left => Location { x: -1, y: 0, z: 0 },
              VirtualKeyCode::Right => Location { x: 1, y: 0, z: 0 },
              _ => unreachable!(),
            };
            let new_delta = delta + delta_change;
            if seen_set.contains(&(game.player_location + new_delta)) {
              display_mode = DisplayMode::ItemTargeting(letter, new_delta);
            }
          }
          _ => {}
        },
      }
    }
    // assumes that the display is wider than tall
    const FOV_DISPLAY_RANGE: i32 = TILE_GRID_WIDTH as i32 / 2;
    // TODO: we should actually only adjust the seen set if the player moved. We
    // should probably make this part of the GameWorld so that it can refresh it
    // when necessary and then we just read that.
    seen_set.clear();
    let z = game.player_location.z;
    ppfov(
      (game.player_location.x, game.player_location.y),
      FOV_DISPLAY_RANGE,
      |x, y| {
        game
          .terrain
          .get(&Location { x, y, z })
          .map(|&t| t == Terrain::Wall || t == Terrain::Ice)
          .unwrap_or(true)
      },
      |x, y| drop(seen_set.insert(Location { x, y, z })),
    );
    {
      match display_mode {
        DisplayMode::Game => draw_game(&mut term, &game, &seen_set),
        DisplayMode::Inventory => draw_inventory(&mut term, &game),
        DisplayMode::ItemTargeting(_letter, delta) => draw_targeting(&mut term, &game, &seen_set, delta),
      }
    }

    unsafe {
      term
        .clear_draw_swap()
        .map_err(|err_vec| {
          for e in err_vec {
            eprintln!("clear_draw_swap error: {:?}", e);
          }
        })
        .ok();
    }
  }
}

fn letter_of(keycode: VirtualKeyCode) -> Option<char> {
  match keycode {
    VirtualKeyCode::A => Some('a'),
    VirtualKeyCode::B => Some('b'),
    VirtualKeyCode::C => Some('c'),
    VirtualKeyCode::D => Some('d'),
    VirtualKeyCode::E => Some('e'),
    VirtualKeyCode::F => Some('f'),
    VirtualKeyCode::G => Some('g'),
    VirtualKeyCode::H => Some('h'),
    VirtualKeyCode::I => Some('i'),
    VirtualKeyCode::J => Some('j'),
    VirtualKeyCode::K => Some('k'),
    VirtualKeyCode::L => Some('l'),
    VirtualKeyCode::M => Some('m'),
    VirtualKeyCode::N => Some('n'),
    VirtualKeyCode::O => Some('o'),
    VirtualKeyCode::P => Some('p'),
    VirtualKeyCode::Q => Some('q'),
    VirtualKeyCode::R => Some('r'),
    VirtualKeyCode::S => Some('s'),
    VirtualKeyCode::T => Some('t'),
    VirtualKeyCode::U => Some('u'),
    VirtualKeyCode::V => Some('v'),
    VirtualKeyCode::W => Some('w'),
    VirtualKeyCode::X => Some('x'),
    VirtualKeyCode::Y => Some('y'),
    VirtualKeyCode::Z => Some('z'),
    _ => None,
  }
}

fn draw_game(term: &mut DwarfTerm, game: &GameWorld, seen_set: &HashSet<Location>) {
  let (mut fgs, mut bgs, mut ids) = term.layer_slices_mut();
  // clear the display
  fgs.set_all(rgb32!(255, 255, 255));
  bgs.set_all(rgb32!(0, 0, 0));
  ids.set_all(0);

  let offset = game.player_location - Location {
    x: (fgs.width() / 2) as i32,
    y: (fgs.height() / 2) as i32,
    z: game.player_location.z,
  };
  // draw the map, save space for the status line.
  const STATUS_HEIGHT: usize = 1;
  let full_extent = (ids.width(), ids.height());
  let map_view_end = (full_extent.0, full_extent.1 - STATUS_HEIGHT);
  for (scr_x, scr_y, id_mut) in ids.slice_mut((0, 0)..map_view_end).iter_mut() {
    let loc_for_this_screen_position = Location {
      x: scr_x as i32,
      y: scr_y as i32,
      z: game.player_location.z,
    } + offset;
    let (glyph, color) = if seen_set.contains(&loc_for_this_screen_position) {
      match game.creature_locations.get(&loc_for_this_screen_position) {
        Some(cid_ref) => {
          let creature_here = game
            .creature_list
            .iter()
            .find(|&creature_ref| &creature_ref.id == cid_ref)
            .expect("Our locations and list are out of sync!");
          (creature_here.icon, creature_here.color)
        }
        None => match game
          .item_locations
          .get(&loc_for_this_screen_position)
          .and_then(|item_vec_ref| item_vec_ref.get(0))
        {
          Some(Item::PotionHealth) => (POTION_GLYPH, rgb32!(250, 5, 5)),
          Some(Item::PotionStrength) => (POTION_GLYPH, rgb32!(5, 240, 20)),
          Some(Item::BombBlast) => (BOMB_GLYPH, rgb32!(127, 127, 127)),
          Some(Item::BombIce) => (BOMB_GLYPH, rgb32!(153, 217, 234)),
          None => match game.terrain.get(&loc_for_this_screen_position) {
            Some(Terrain::Wall) => (WALL_TILE, rgb32!(155, 75, 0)),
            Some(Terrain::Ice) => (WALL_TILE, rgb32!(112, 146, 190)),
            Some(Terrain::Floor) => (b'.', rgb32!(128, 128, 128)),
            Some(Terrain::StairsDown) => (b'>', rgb32!(190, 190, 190)),
            Some(Terrain::StairsUp) => (b'<', rgb32!(190, 190, 190)),
            None => (b' ', 0),
          },
        },
      }
    } else {
      (b' ', 0)
    };
    *id_mut = glyph;
    fgs[(scr_x, scr_y)] = color;
  }
  // draw the status bar.
  fgs.slice_mut((0, map_view_end.1)..full_extent).set_all(KINDA_LIME_GREEN);
  bgs.slice_mut((0, map_view_end.1)..full_extent).set_all(rgb32!(0, 0, 0));
  let mut ids_status_slice_mut = ids.slice_mut((0, map_view_end.1)..full_extent);
  debug_assert_eq!(ids_status_slice_mut.width(), full_extent.0);
  debug_assert_eq!(ids_status_slice_mut.height(), STATUS_HEIGHT);
  ids_status_slice_mut.set_all(0);
  debug_assert_eq!(1, STATUS_HEIGHT);
  let mut status_line_u8_slice_mut: &mut [u8] = unsafe { ::std::slice::from_raw_parts_mut(ids_status_slice_mut.as_mut_ptr(), full_extent.0) };
  let player_hp = game
    .creature_list
    .iter()
    .find(|creature_ref| creature_ref.is_the_player)
    .unwrap()
    .hit_points;
  write!(
    status_line_u8_slice_mut,
    "HP: {}, Enemies: {}, Z:{}",
    player_hp,
    game.creature_list.len() - 1,
    game.player_location.z
  ).ok();
}

fn draw_inventory(term: &mut DwarfTerm, game: &GameWorld) {
  let (mut fgs, mut bgs, mut ids) = term.layer_slices_mut();
  // clear the display
  fgs.set_all(rgb32!(255, 255, 255));
  bgs.set_all(rgb32!(0, 0, 0));
  ids.set_all(0);

  let mut map_item_count = BTreeMap::new();
  for item_ref in game
    .creature_list
    .iter()
    .find(|creature_ref| creature_ref.is_the_player)
    .unwrap()
    .inventory
    .iter()
  {
    *map_item_count.entry(item_ref).or_insert(0) += 1;
  }

  let mut item_list = vec![];
  for (key, val) in map_item_count.into_iter() {
    match val {
      0 => panic!("what the heck?"),
      1 => item_list.push(format!("{}", key)),
      count => item_list.push(format!("{} ({})", key, count)),
    }
  }

  // draw the menu title
  {
    let menu_title = "== Inventory ==";
    assert!(menu_title.len() < ids.width());
    let x_offset = (ids.width() - menu_title.len()) as isize / 2;
    let y_offset = (ids.height() as isize - 1) as isize;
    let mut this_line_slice_mut: &mut [u8] =
      unsafe { ::std::slice::from_raw_parts_mut(ids.as_mut_ptr().offset(x_offset + y_offset * ids.pitch()), menu_title.len()) };
    write!(this_line_slice_mut, "{}", menu_title).ok();
  }
  // draw the items
  if item_list.len() > 0 {
    let mut the_y_position: isize = ids.height() as isize - 2;
    for (i, item) in item_list.into_iter().enumerate() {
      if the_y_position < 0 {
        break;
      }
      let mut this_line_slice_mut: &mut [u8] =
        unsafe { ::std::slice::from_raw_parts_mut(ids.as_mut_ptr().offset(ids.pitch() * the_y_position), ids.width()) };
      let letter = i + ('a' as u8 as usize);
      write!(this_line_slice_mut, "{}) {}", letter as u8 as char, item).ok();
      the_y_position -= 1;
    }
  } else {
    let message = "You have no items on hand.";
    assert!(message.len() < ids.width());
    let x_offset = (ids.width() - message.len()) as isize / 2;
    let y_offset = (ids.height() as isize - 3) as isize;
    let mut this_line_slice_mut: &mut [u8] =
      unsafe { ::std::slice::from_raw_parts_mut(ids.as_mut_ptr().offset(x_offset + y_offset * ids.pitch()), message.len()) };
    write!(this_line_slice_mut, "{}", message).ok();
  }
}

fn draw_targeting(term: &mut DwarfTerm, game: &GameWorld, seen_set: &HashSet<Location>, delta: Location) {
  let (mut fgs, mut bgs, mut ids) = term.layer_slices_mut();
  // clear the display
  fgs.set_all(rgb32!(255, 255, 255));
  bgs.set_all(rgb32!(0, 0, 0));
  ids.set_all(0);

  // draw the menu title
  {
    let menu_title = "== Select A Target ==";
    assert!(menu_title.len() < ids.width());
    let x_offset = (ids.width() - menu_title.len()) as isize / 2;
    let y_offset = (ids.height() as isize - 1) as isize;
    let mut this_line_slice_mut: &mut [u8] =
      unsafe { ::std::slice::from_raw_parts_mut(ids.as_mut_ptr().offset(x_offset + y_offset * ids.pitch()), menu_title.len()) };
    write!(this_line_slice_mut, "{}", menu_title).ok();
  }

  let offset = game.player_location - Location {
    x: (fgs.width() / 2) as i32,
    y: (fgs.height() / 2) as i32,
    z: game.player_location.z,
  };
  let target_delta_location = game.player_location + delta;
  // draw the map, save space for the status line.
  const STATUS_HEIGHT: usize = 1;
  let full_extent = (ids.width(), ids.height());
  let map_view_end = (full_extent.0, full_extent.1 - STATUS_HEIGHT);
  for (scr_x, scr_y, id_mut) in ids.slice_mut((0, 0)..map_view_end).iter_mut() {
    let loc_for_this_screen_position = Location {
      x: scr_x as i32,
      y: scr_y as i32,
      z: game.player_location.z,
    } + offset;
    let (glyph, color) = if seen_set.contains(&loc_for_this_screen_position) {
      match game.creature_locations.get(&loc_for_this_screen_position) {
        Some(cid_ref) => {
          let creature_here = game
            .creature_list
            .iter()
            .find(|&creature_ref| &creature_ref.id == cid_ref)
            .expect("Our locations and list are out of sync!");
          (creature_here.icon, creature_here.color)
        }
        None => match game
          .item_locations
          .get(&loc_for_this_screen_position)
          .and_then(|item_vec_ref| item_vec_ref.get(0))
        {
          Some(Item::PotionHealth) => (POTION_GLYPH, rgb32!(250, 5, 5)),
          Some(Item::PotionStrength) => (POTION_GLYPH, rgb32!(5, 240, 20)),
          Some(Item::BombBlast) => (BOMB_GLYPH, rgb32!(127, 127, 127)),
          Some(Item::BombIce) => (BOMB_GLYPH, rgb32!(153, 217, 234)),
          None => match game.terrain.get(&loc_for_this_screen_position) {
            Some(Terrain::Wall) => (WALL_TILE, rgb32!(155, 75, 0)),
            Some(Terrain::Ice) => (WALL_TILE, rgb32!(112, 146, 190)),
            Some(Terrain::Floor) => (b'.', rgb32!(128, 128, 128)),
            Some(Terrain::StairsDown) => (b'>', rgb32!(190, 190, 190)),
            Some(Terrain::StairsUp) => (b'<', rgb32!(190, 190, 190)),
            None => (b' ', 0),
          },
        },
      }
    } else {
      (b' ', 0)
    };
    *id_mut = glyph;
    fgs[(scr_x, scr_y)] = color;
    if loc_for_this_screen_position == target_delta_location {
      const FULL_ALPHA: u32 = rgba32!(0, 0, 0, 255);
      fgs[(scr_x, scr_y)] = !fgs[(scr_x, scr_y)] | FULL_ALPHA;
      bgs[(scr_x, scr_y)] = !bgs[(scr_x, scr_y)] | FULL_ALPHA;
    }
  }
}

fn save_game(game: &GameWorld) -> std::io::Result<()> {
  let mut f = std::fs::File::create("kasidin.save")?;
  let encoded: Vec<u8> =
    bincode::serialize(&game).map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "Couldn't serialize the game!"))?;
  f.write_all(&encoded)
}

fn load_game(game: &mut GameWorld) -> std::io::Result<()> {
  let mut f = std::fs::File::open("kasidin.save")?;
  let mut file_bytes: Vec<u8> = vec![];
  f.read_to_end(&mut file_bytes)?;
  let decoded: GameWorld =
    bincode::deserialize(&file_bytes).map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "Couldn't deserialize the game!"))?;
  *game = decoded;
  Ok(())
}
