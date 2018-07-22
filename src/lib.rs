#![feature(nll)]
#![allow(unused_mut)]

extern crate dwarf_term;
pub(crate) use dwarf_term::*;

extern crate serde;

#[macro_use]
extern crate serde_derive;

// std
pub(crate) use std::collections::hash_map::*;
pub(crate) use std::collections::hash_set::*;
pub(crate) use std::collections::BTreeMap;
pub(crate) use std::ops::*;

pub mod pathing;
pub use pathing::*;
pub mod precise_permissive_fov;
pub use precise_permissive_fov::*;
pub mod prng;
pub use prng::*;

pub const WALL_TILE: u8 = 11 + 13 * 16;
pub const POTION_GLYPH: u8 = 13 + 10 * 16;
pub const BOMB_GLYPH: u8 = 15 + 0 * 16;

pub const TERULO_BROWN: u32 = rgb32!(197, 139, 5);
pub const KESTREL_RED: u32 = rgb32!(166, 0, 0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Item {
  PotionHealth,
  PotionStrength,
  BombBlast,
  BombIce,
}

impl Item {
  fn is_potion(self) -> bool {
    match self {
      Item::PotionHealth | Item::PotionStrength => true,
      _ => false,
    }
  }
}

fn apply_potion(potion: &Item, target: &mut Creature, rng: &mut PCG32) {
  match potion {
    Item::PotionHealth => target.hit_points = (target.hit_points + step(rng, 8)).min(30),
    Item::PotionStrength => target.damage_step += 1,
    _ => panic!("not a potion {}", potion),
  }
}

impl ::std::fmt::Display for Item {
  fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
    match self {
      Item::PotionHealth => write!(f, "Potion of Restore Health"),
      Item::PotionStrength => write!(f, "Potion of Gain Strength"),
      Item::BombBlast => write!(f, "Blast Bomb"),
      Item::BombIce => write!(f, "Ice Bomb"),
    }
  }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default, Hash, Serialize, Deserialize)]
pub struct Location {
  pub x: i32,
  pub y: i32,
  pub z: i32,
}

/// Iterates over the 4 cardinal directions.
struct LocationNeighborsIter {
  x: i32,
  y: i32,
  z: i32,
  index: usize,
}
impl Iterator for LocationNeighborsIter {
  type Item = Location;
  fn next(&mut self) -> Option<Self::Item> {
    match self.index {
      0 => {
        self.index += 1;
        Some(Location {
          x: self.x + 1,
          y: self.y,
          z: self.z,
        })
      }
      1 => {
        self.index += 1;
        Some(Location {
          x: self.x - 1,
          y: self.y,
          z: self.z,
        })
      }
      2 => {
        self.index += 1;
        Some(Location {
          x: self.x,
          y: self.y + 1,
          z: self.z,
        })
      }
      3 => {
        self.index += 1;
        Some(Location {
          x: self.x,
          y: self.y - 1,
          z: self.z,
        })
      }
      _ => None,
    }
  }
}

impl Location {
  pub fn neighbors(&self) -> impl Iterator<Item = Location> {
    LocationNeighborsIter {
      x: self.x,
      y: self.y,
      z: self.z,
      index: 0,
    }
  }
}

impl Add for Location {
  type Output = Self;
  fn add(self, other: Self) -> Self {
    Location {
      x: self.x + other.x,
      y: self.y + other.y,
      z: self.z + other.z,
    }
  }
}

impl Sub for Location {
  type Output = Self;
  fn sub(self, other: Self) -> Self {
    Location {
      x: self.x - other.x,
      y: self.y - other.y,
      z: self.z - other.z,
    }
  }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Creature {
  pub icon: u8,
  pub color: u32,
  pub is_the_player: bool,
  pub id: CreatureID,
  pub hit_points: i32,
  pub damage_step: i32,
  pub inventory: Vec<Item>,
}
impl Creature {
  fn new(cid: usize, icon: u8, color: u32) -> Self {
    Creature {
      icon,
      color,
      is_the_player: false,
      id: CreatureID(cid),
      hit_points: 1,
      damage_step: 1,
      inventory: vec![],
    }
  }

  fn new_player(cid: usize) -> Self {
    let mut out = Self::new(cid, b'@', TERULO_BROWN);
    out.is_the_player = true;
    out.hit_points = 20;
    out.damage_step = 5;
    out
  }

  fn new_kestrel(cid: usize) -> Self {
    let mut out = Self::new(cid, b'k', KESTREL_RED);
    out.hit_points = 8;
    out.damage_step = 3;
    out
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Terrain {
  Wall,
  Floor,
  Ice,
  StairsDown,
  StairsUp,
}

impl Default for Terrain {
  fn default() -> Self {
    Terrain::Wall
  }
}

fn make_cellular_caves(width: usize, height: usize, gen: &mut PCG32) -> VecImage<bool> {
  // utilities
  let range_count = |buf: &VecImage<bool>, x: usize, y: usize, range: u32| {
    debug_assert!(range > 0);
    let mut total = 0;
    for y in ((y as isize - range as isize) as usize)..=(y + range as usize) {
      for x in ((x as isize - range as isize) as usize)..=(x + range as usize) {
        if y == 0 && x == 0 {
          continue;
        } else {
          match buf.get((x, y)) {
            Some(&b) => if b {
              total += 1;
            },
            None => {
              total += 1;
            }
          }
        }
      }
    }
    total
  };
  let cave_copy = |src: &VecImage<bool>, dest: &mut VecImage<bool>| {
    for (x, y, mut_ref) in dest.iter_mut() {
      // TODO: this will count up some of the cells more than once, perhaps we
      // can make this more efficient by making it more fiddly.
      *mut_ref = range_count(src, x, y, 1) >= 5 || range_count(src, x, y, 2) <= 1;
    }
  };
  let flood_copy = |src: &VecImage<bool>, dest: &mut VecImage<bool>, gen: &mut PCG32| {
    dest.set_all(true);
    let mut copied_count = 0;
    let start = {
      let d_width = RandRangeInclusive32::new(0..=((width - 1) as u32));
      let d_height = RandRangeInclusive32::new(0..=((height - 1) as u32));
      let mut x = d_width.roll_with(gen) as usize;
      let mut y = d_height.roll_with(gen) as usize;
      let mut tries = 0;
      while src[(x, y)] {
        x = d_width.roll_with(gen) as usize;
        y = d_height.roll_with(gen) as usize;
        tries += 1;
        if tries > 100 {
          return 0;
        }
      }
      (x, y)
    };
    let mut open_set = HashSet::new();
    let mut closed_set = HashSet::new();
    open_set.insert(start);
    while !open_set.is_empty() {
      let loc: (usize, usize) = *open_set.iter().next().unwrap();
      open_set.remove(&loc);
      if closed_set.contains(&loc) {
        continue;
      } else {
        closed_set.insert(loc);
      };
      if !src[loc] {
        dest[loc] = false;
        copied_count += 1;
        if loc.0 > 1 {
          open_set.insert((loc.0 - 1, loc.1));
        }
        if loc.0 < (src.width() - 2) {
          open_set.insert((loc.0 + 1, loc.1));
        }
        if loc.1 > 1 {
          open_set.insert((loc.0, loc.1 - 1));
        }
        if loc.1 < (src.height() - 2) {
          open_set.insert((loc.0, loc.1 + 1));
        }
      }
    }
    copied_count
  };

  let d100 = RandRangeInclusive32::new(1..=100);
  let mut buffer_a: VecImage<bool> = VecImage::new(width, height);
  let mut buffer_b: VecImage<bool> = VecImage::new(width, height);

  'work: loop {
    // fill the initial buffer, all cells 45% likely.
    for (_x, _y, mut_ref) in buffer_a.iter_mut() {
      *mut_ref = d100.roll_with(gen) <= 45;
    }
    // cave copy from A into B, then the reverse, 5 times total
    cave_copy(&buffer_a, &mut buffer_b);
    cave_copy(&buffer_b, &mut buffer_a);
    cave_copy(&buffer_a, &mut buffer_b);
    cave_copy(&buffer_b, &mut buffer_a);
    cave_copy(&buffer_a, &mut buffer_b);
    // good stuff is in B, flood copy back into A
    let copied_count = flood_copy(&buffer_b, &mut buffer_a, gen);
    if copied_count >= (width * height) / 2 {
      return buffer_a;
    } else {
      continue 'work;
    }
  }
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CreatureID(pub usize);

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct GameWorld {
  pub player_location: Location,
  pub next_creature_id: usize,
  pub creature_list: Vec<Creature>,
  pub creature_locations: HashMap<Location, CreatureID>,
  pub item_locations: HashMap<Location, Vec<Item>>,
  pub terrain: HashMap<Location, Terrain>,
  pub gen: PCG32,
  pub deepest_depth: i32,
}
const GAME_DIMENSIONS: usize = 50;

impl GameWorld {
  pub fn new(seed: u64) -> Self {
    // Make our world
    let mut out = Self {
      player_location: Location { x: 0, y: 0, z: 0 },
      next_creature_id: 1,
      creature_list: vec![],
      creature_locations: HashMap::new(),
      item_locations: HashMap::new(),
      terrain: HashMap::new(),
      gen: PCG32::new(seed),
      deepest_depth: 1,
    };

    // Generate the player
    let mut player = Creature::new_player(out.next_creature_id);
    out.next_creature_id += 1;

    // Add the first z-layer.
    out.add_z_layer(None);

    // Place the Player
    let player_start = out.pick_random_floor(out.deepest_depth);
    let player_id = player.id.0;
    out.creature_list.push(player);
    out.creature_locations.insert(player_start, CreatureID(player_id));
    out.player_location = player_start;

    out
  }

  pub fn add_z_layer(&mut self, down_stairs: Option<Location>) {
    self.deepest_depth -= 1;

    // Generate a new z layer, with optional constraint
    let caves: VecImage<bool> = match down_stairs {
      None => make_cellular_caves(GAME_DIMENSIONS, GAME_DIMENSIONS, &mut self.gen),
      Some(stairs) => 'cave: loop {
        let potential = make_cellular_caves(GAME_DIMENSIONS, GAME_DIMENSIONS, &mut self.gen);
        let center = (stairs.x as usize, stairs.y as usize);
        for xy in Some(center)
          .into_iter()
          .chain(stairs.neighbors().map(|loc| (loc.x as usize, loc.y as usize)))
        {
          if !potential[xy] {
            break 'cave potential;
          }
        }
      },
    };

    // Place the Terrain
    for (x, y, tile) in caves.iter() {
      self.terrain.insert(
        Location {
          x: x as i32,
          y: y as i32,
          z: self.deepest_depth,
        },
        if *tile { Terrain::Wall } else { Terrain::Floor },
      );
    }
    // Add the stairs back up, if necessary
    down_stairs.map(|loc| self.terrain.insert(loc + Location { x: 0, y: 0, z: -1 }, Terrain::StairsUp));
    // Add some stairs even deeper
    let stairs_place = self.pick_random_floor(self.deepest_depth);
    self.terrain.insert(stairs_place, Terrain::StairsDown);

    // Place the Creatures
    for _ in 0..(GAME_DIMENSIONS / 2) {
      let monster = Creature::new_kestrel(self.next_creature_id);
      self.next_creature_id += 1;
      let monster_id = monster.id.0;
      let monster_start = self.pick_random_floor(self.deepest_depth);
      match self.creature_locations.entry(monster_start) {
        Entry::Occupied(_) => {
          // if we happen to pick an occupied location, just don't add a
          // creature for this pass of the loop.
          continue;
        }
        Entry::Vacant(ve) => {
          self.creature_list.push(monster);
          ve.insert(CreatureID(monster_id));
        }
      }
    }

    // Place the Items
    for _ in 0..GAME_DIMENSIONS {
      let item_spot = self.pick_random_floor(self.deepest_depth);
      let new_item = match self.gen.next_u32() >> 30 {
        0 => Item::PotionHealth,
        1 => Item::PotionStrength,
        2 => Item::BombBlast,
        3 => Item::BombIce,
        _ => unreachable!(),
      };
      self.item_locations.entry(item_spot).or_insert(Vec::new()).push(new_item);
    }
  }

  pub fn pick_random_floor(&mut self, z: i32) -> Location {
    let indexer = RandRangeInclusive32::new(0..=(GAME_DIMENSIONS as u32 - 1));
    let mut tries = 0;
    let mut x = indexer.roll_with(&mut self.gen) as usize;
    let mut y = indexer.roll_with(&mut self.gen) as usize;
    let mut loc = Location { x: x as i32, y: y as i32, z };
    while self.terrain[&loc] != Terrain::Floor {
      x = indexer.roll_with(&mut self.gen) as usize;
      y = indexer.roll_with(&mut self.gen) as usize;
      loc = Location { x: x as i32, y: y as i32, z };
      if tries > 5000 {
        panic!("couldn't find a floor tile!");
      }
    }
    loc
  }

  pub fn move_player(&mut self, delta: Location) {
    let player_move_target = self.player_location + delta;
    match self.creature_locations.get(&player_move_target) {
      Some(target_id_ref) => {
        // someone is there, do the attack!
        let player_damage_roll = {
          let player_id_ref = self.creature_locations.get(&self.player_location).unwrap();
          let player_ref = self.creature_list.iter().find(|creature_ref| &creature_ref.id == player_id_ref).unwrap();
          step(&mut self.gen, player_ref.damage_step)
        };
        let target_ref_mut = self
          .creature_list
          .iter_mut()
          .find(|creature_mut_ref| &creature_mut_ref.id == target_id_ref)
          .unwrap();
        target_ref_mut.hit_points -= player_damage_roll;
        println!("Player did {} damage to {:?}", player_damage_roll, target_id_ref);
      }
      None => {
        // no one is there, move
        match *self.terrain.entry(player_move_target).or_insert(Terrain::Floor) {
          Terrain::Wall | Terrain::Ice => {
            // Accidentally bumping a wall doesn't consume a turn.
            return;
          }
          Terrain::Floor | Terrain::StairsDown | Terrain::StairsUp => {
            let player_id = self
              .creature_locations
              .remove(&self.player_location)
              .expect("The player wasn't where they should be!");
            let old_creature = self.creature_locations.insert(player_move_target, player_id);
            debug_assert!(old_creature.is_none());
            self.player_location = player_move_target;
            // grab items that are here, if any
            let player_id_ref = self.creature_locations.get(&self.player_location).unwrap();
            let player_mut = self
              .creature_list
              .iter_mut()
              .find(|creature_mut| &creature_mut.id == player_id_ref)
              .unwrap();
            let floor_items = self.item_locations.entry(self.player_location).or_insert(Vec::new());
            player_mut.inventory.append(floor_items);
          }
        }
      }
    }
    self.run_world_turn();
  }

  pub fn change_floor(&mut self, floor_delta: i32) {
    let player_terrain = self.terrain[&self.player_location];
    match (player_terrain, floor_delta) {
      (Terrain::StairsDown, -1) => {
        if self.player_location.z == self.deepest_depth {
          self.add_z_layer(Some(self.player_location));
        }
        self.move_player(Location { x: 0, y: 0, z: -1 })
      }
      (Terrain::StairsUp, 1) => self.move_player(Location { x: 0, y: 0, z: 1 }),
      _ => {}
    }
  }

  pub fn use_item(&mut self, item_letter: char) -> UseItemResult {
    let player_mut = self.creature_list.iter_mut().find(|creature_ref| creature_ref.is_the_player).unwrap();
    let item_to_use = {
      let mut cataloged_inventory = BTreeMap::new();
      for item_ref in player_mut.inventory.iter() {
        *cataloged_inventory.entry(item_ref).or_insert(0) += 1;
      }
      let letter_index = item_letter as u8 - 'a' as u8;
      cataloged_inventory.into_iter().nth(letter_index as usize).map(|(&item, _count)| item)
    };
    match item_to_use {
      Some(item) => {
        if item.is_potion() {
          apply_potion(&item, player_mut, &mut self.gen);
          for i in 0..player_mut.inventory.len() {
            if player_mut.inventory[i] == item {
              player_mut.inventory.remove(i);
              break;
            }
          }
          self.run_world_turn();
          UseItemResult::ItemUsed
        } else {
          UseItemResult::ItemNeedsTarget
        }
      }
      None => UseItemResult::NoSuchItem,
    }
  }

  pub fn use_targeted_item(&mut self, item_letter: char, target_delta: Location) {
    let item_to_use = {
      let player_mut = self.creature_list.iter_mut().find(|creature_ref| creature_ref.is_the_player).unwrap();
      let mut cataloged_inventory = BTreeMap::new();
      for item_ref in player_mut.inventory.iter() {
        *cataloged_inventory.entry(item_ref).or_insert(0) += 1;
      }
      let letter_index = item_letter as u8 - 'a' as u8;
      cataloged_inventory.into_iter().nth(letter_index as usize).map(|(&item, _count)| item)
    };

    match item_to_use {
      Some(Item::BombBlast) => {
        let mut blast_locations = vec![];
        let blast_center = self.player_location + target_delta;
        let z = self.player_location.z;
        ppfov(
          (blast_center.x, blast_center.y),
          2,
          |x, y| self.terrain[&Location { x, y, z }] == Terrain::Wall,
          |x, y| blast_locations.push(Location { x, y, z }),
        );
        let mut blast_targets = vec![];
        for location in blast_locations.into_iter() {
          if *self.terrain.entry(location).or_insert(Terrain::Wall) == Terrain::Ice {
            *self.terrain.entry(location).or_insert(Terrain::Wall) = Terrain::Floor;
          }
          match self.creature_locations.get(&location) {
            None => {}
            Some(cid_ref) => {
              blast_targets.push(CreatureID(cid_ref.0));
            }
          }
        }
        for creature_mut in self.creature_list.iter_mut() {
          if blast_targets.contains(&creature_mut.id) {
            creature_mut.hit_points -= step(&mut self.gen, 10);
          }
        }
      }
      Some(Item::BombIce) => {
        let mut blast_locations = vec![];
        let blast_center = self.player_location + target_delta;
        let z = self.player_location.z;
        ppfov(
          (blast_center.x, blast_center.y),
          1,
          |_, _| false, /* vision check doesn't matter on radius 1 fov */
          |x, y| blast_locations.push(Location { x, y, z }),
        );
        for location in blast_locations.into_iter() {
          if *self.terrain.entry(location).or_insert(Terrain::Wall) == Terrain::Floor {
            *self.terrain.entry(location).or_insert(Terrain::Wall) = Terrain::Ice;
            self.item_locations.entry(location).or_insert(Vec::new()).clear();
            let removed_cid = self.creature_locations.remove(&location);
            // this is a hacky way to never delete the player on accident, but
            // not really any _more_ hacky than the rest of the codebase.
            removed_cid.map(|cid_ref| {
              if cid_ref.0 > 1 {
                self.creature_list.retain(|creature_ref| &creature_ref.id != &cid_ref);
              } else {
                self.creature_locations.insert(location, CreatureID(cid_ref.0));
              }
            });
          }
        }
      }
      Some(other) => panic!("Item was not an item that can target: {}", other),
      None => panic!("No such item letter: {}", item_letter),
    }
    let item_used = item_to_use.unwrap();
    let player_mut = self.creature_list.iter_mut().find(|creature_ref| creature_ref.is_the_player).unwrap();
    for i in 0..player_mut.inventory.len() {
      if player_mut.inventory[i] == item_used {
        player_mut.inventory.remove(i);
        break;
      }
    }
    self.run_world_turn();
  }

  pub fn run_world_turn(&mut self) {
    let initiative_list: Vec<CreatureID> = self
      .creature_list
      .iter()
      .filter_map(|creature_mut| {
        if creature_mut.is_the_player || creature_mut.hit_points < 1 {
          None
        } else {
          Some(CreatureID(creature_mut.id.0))
        }
      })
      .collect();
    for creature_id_ref in initiative_list.iter() {
      let my_location: Option<Location> = {
        self
          .creature_locations
          .iter()
          .find(|&(_loc, id)| id == creature_id_ref)
          .map(|(&loc, _id)| loc)
      };
      match my_location {
        None => println!("Creature {:?} is not anywhere!", creature_id_ref),
        Some(loc) => {
          // Look around
          let seen_locations = {
            let terrain_ref = &self.terrain;
            let mut seen_locations = HashSet::new();
            let z = loc.z;
            ppfov(
              (loc.x, loc.y),
              7,
              |x, y| {
                let here = *terrain_ref.get(&Location { x, y, z }).unwrap_or(&Terrain::Wall);
                here == Terrain::Wall || here == Terrain::Ice
              },
              |x, y| {
                seen_locations.insert(Location { x, y, z });
              },
            );
            seen_locations
          };
          // Decide where to go
          let move_target = if seen_locations.contains(&self.player_location) {
            let terrain_ref = &self.terrain;
            let path = a_star(self.player_location, loc, |loc| {
              terrain_ref.get(&loc).unwrap_or(&Terrain::Wall) != &Terrain::Wall
            }).expect("couldn't find a path");
            debug_assert_eq!(loc, path[0]);
            path[1]
          } else {
            loc + match self.gen.next_u32() >> 30 {
              0 => Location { x: 0, y: 1, z: 0 },
              1 => Location { x: 0, y: -1, z: 0 },
              2 => Location { x: 1, y: 0, z: 0 },
              3 => Location { x: -1, y: 0, z: 0 },
              impossible => unreachable!("u32 >> 30: {}", impossible),
            }
          };
          // go there
          match self.creature_locations.get(&move_target) {
            Some(target_id_ref) => {
              // someone is there, do the attack!
              let creature_damage_roll = {
                let creature_ref = self
                  .creature_list
                  .iter()
                  .find(|creature_ref| &creature_ref.id == creature_id_ref)
                  .unwrap();
                step(&mut self.gen, creature_ref.damage_step)
              };
              let target_ref_mut = self
                .creature_list
                .iter_mut()
                .find(|creature_mut_ref| &creature_mut_ref.id == target_id_ref)
                .unwrap();
              if target_ref_mut.is_the_player {
                target_ref_mut.hit_points -= creature_damage_roll;
                println!("{:?} did {} damage to {:?}", creature_id_ref, creature_damage_roll, target_id_ref);
              }
              // TODO: log that we did damage.
            }
            None => match *self.terrain.entry(move_target).or_insert(Terrain::Floor) {
              Terrain::Wall | Terrain::Ice => {
                continue;
              }
              Terrain::Floor | Terrain::StairsDown | Terrain::StairsUp => {
                let id = self.creature_locations.remove(&loc).expect("The creature wasn't where they should be!");
                let old_id = self.creature_locations.insert(move_target, id);
                debug_assert!(old_id.is_none());
              }
            },
          }
        }
      }
    }
    // End Phase, we clear any dead NPCs off the list.
    let creature_locations_mut = &mut self.creature_locations;
    self.creature_list.retain(|creature_ref| {
      let keep = creature_ref.hit_points > 0 || creature_ref.is_the_player;
      if !keep {
        let dead_location = *creature_locations_mut
          .iter()
          .find(|&(_, v_cid)| v_cid == &creature_ref.id)
          .expect("Locations list out of sync!")
          .0;
        creature_locations_mut.remove(&dead_location);
      };
      keep
    });
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UseItemResult {
  ItemUsed,
  ItemNeedsTarget,
  NoSuchItem,
}
