
# Week 07

## Part 12: Deeper Dangers

Eh, so, this is in the progression chart of the "Official Roguelike Tutorial",
but it's actually not interesting at all. Also, I'm not a game designer so I
don't want to focus too much on that anyway. We'll just make it so that the
monsters get the dungeon depth added to their hit points when they're created.

```rust
  fn new_kestrel(cid: usize, depth: i32) -> Self {
    let mut out = Self::new(cid, b'k', KESTREL_RED);
    out.hit_points = 8 + depth.abs();
    out.damage_step = 3;
    out
  }
```

And we change how we call that.

```rust
// part of GameWorld::add_z_layer
    for _ in 0..(GAME_DIMENSIONS / 2) {
      let monster = Creature::new_kestrel(self.next_creature_id, self.deepest_depth);
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
```

And we're done.

That's it, that's the whole thing for part 12. I encourage you to do your own
difficulty scaling, but you already get the idea. Just pass more params to
whatever digs each deeper level (or builds each higher level if it's a "tower"
dungeon, or re-spawns zones in an open world thing, and so on).

## Part 13: Equipment

So we'll add weapons to hold and armor to wear. Let's have more than one kind of
thing for each of those, so that we can make deeper levels shift the probability
of a given kind of thing as you go down. What kinds of weapons does a Terulo
use? I guess just a Dagger for the "small" one and a Warhammer for the "big"
one. For armor, let's go with a "light", "medium", and "heavy" split. So
that's... Fernweave, Lobster Mail, and Crystal Plate. That sounds sufficiently
flavorful. Astute readers will note that I actually just stole those weird
armors from not one but _two_ different semi-obscure table-top games this time
around.

So, we're adding new Items to the game. Let's add them to the Item enum and see
what breaks. This is our new Item type:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Item {
  PotionHealth,
  PotionStrength,
  BombBlast,
  BombIce,
  Dagger(i8),
  Warhammer(i8),
  Fernweave(i8),
  LobsterMail(i8),
  CrystalPlate(i8),
}
```

Wait, what? What's all those `i8` values doing there? Of course, in Rust you can
have a fields within an enum variant (which maybe you didn't know), but what do
they mean? Well, of course there's usually a "bonus" on a weapon, like a Dagger
+2 or a Warhammer -1, and similar for armor. They're almost always in a tiny
range around 0, like +5 to -5, but `i8` is the smallest value we can reasonably
store without delving into bit fiddling, so we'll make them use `i8`.

Well what breaks? Hmm, suspiciously not too much. Looks like we'll need to
expand on how `Display` works, and... that's about it. Hmm. Ah, yes, placing
items will need an update of course, but it went the other way (RNG roll to item
type) so the fact that we don't place every type of item didn't cause any
compile error. Similarly, the item use code makes a call to `is_potion` and then
assumes that anything that's not a potion is an item that you have to target.
Bit of a gaff there perhaps. Still, that's not too bad.

```rust
impl ::std::fmt::Display for Item {
  fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
    match self {
      Item::PotionHealth => write!(f, "Potion of Restore Health"),
      Item::PotionStrength => write!(f, "Potion of Gain Strength"),
      Item::BombBlast => write!(f, "Blast Bomb"),
      Item::BombIce => write!(f, "Ice Bomb"),
      Item::Dagger(x) => write!(f, "Dagger {}{}", if *x >= 0 { "+" } else { "-" }, x),
      Item::Warhammer(x) => write!(f, "Warhammer {}{}", if *x >= 0 { "+" } else { "-" }, x),
      Item::Fernweave(x) => write!(f, "Fernweave {}{}", if *x >= 0 { "+" } else { "-" }, x),
      Item::LobsterMail(x) => write!(f, "Lobster Mail {}{}", if *x >= 0 { "+" } else { "-" }, x),
      Item::CrystalPlate(x) => write!(f, "Crystal Plate {}{}", if *x >= 0 { "+" } else { "-" }, x),
    }
  }
}
```

Boring, but effective.

Now our game display needs a way to show these new items too. We're still doing
that in two places, so let's pull it out into a function because that's getting
dumb.

So, in both `draw_game` and `draw_targeting` we'll replace all those matches
with this ugly Option juggling thing:

```rust
    let (glyph, color) = if seen_set.contains(&loc_for_this_screen_position) {
      game
        .creature_locations
        .get(&loc_for_this_screen_position)
        .map(|cid_ref| {
          let creature_here = game
            .creature_list
            .iter()
            .find(|&creature_ref| &creature_ref.id == cid_ref)
            .expect("Our locations and list are out of sync!");
          (creature_here.icon, creature_here.color)
        })
        .unwrap_or_else(|| {
          game
            .item_locations
            .get(&loc_for_this_screen_position)
            .and_then(|item_vec_ref| item_vec_ref.get(0))
            .map(|item_ref| display_of_item(*item_ref))
            .unwrap_or_else(|| {
              game
                .terrain
                .get(&loc_for_this_screen_position)
                .map(|terrain_ref| display_of_terrain(*terrain_ref))
                .unwrap_or((b' ', 0))
            })
        })
    } else {
      (b' ', 0)
    };
```

Now we keep the code to determine the display stuff all in one place:

```rust
fn display_of_item(item: Item) -> (u8, u32) {
  match item {
    Item::PotionHealth => (POTION_GLYPH, rgb32!(250, 5, 5)),
    Item::PotionStrength => (POTION_GLYPH, rgb32!(5, 240, 20)),
    Item::BombBlast => (BOMB_GLYPH, rgb32!(127, 127, 127)),
    Item::BombIce => (BOMB_GLYPH, rgb32!(153, 217, 234)),
    Item::CrystalPlate(_) => (ARMOR_GLYPH, rgb32!(0, 162, 232)),
    Item::LobsterMail(_) => (ARMOR_GLYPH, rgb32!(237, 28, 36)),
    Item::Fernweave(_) => (ARMOR_GLYPH, rgb32!(34, 177, 76)),
    Item::Dagger(_) => (WEAPON_GLYPH, rgb32!(195, 195, 195)),
    Item::Warhammer(_) => (WEAPON_GLYPH, rgb32!(127, 127, 127)),
  }
}

fn display_of_terrain(terrain: Terrain) -> (u8, u32) {
  match terrain {
    Terrain::Wall => (WALL_TILE, rgb32!(155, 75, 0)),
    Terrain::Ice => (WALL_TILE, rgb32!(112, 146, 190)),
    Terrain::Floor => (b'.', rgb32!(128, 128, 128)),
    Terrain::StairsDown => (b'>', rgb32!(190, 190, 190)),
    Terrain::StairsUp => (b'<', rgb32!(190, 190, 190)),
  }
}
```

And we need to drop some on the ground during item placement. Well, we want the
frequency of things to change with depth, so I guess we need a structure to take
a lot of things and frequencies for them and then give outputs.

Here's my plan:

```rust
#[derive(Debug, Clone)]
pub struct FrequencyChart<T: Clone> {
  rand_range: RandRangeInclusive32,
  items: Vec<(u32, T)>,
}

impl<T: Clone> FrequencyChart<T> {
  pub fn new(item: T, frequency: u32) -> Self {
    Self {
      rand_range: RandRangeInclusive32::new(1..=frequency.max(1)),
      items: vec![(frequency, item)],
    }
  }

  pub fn push_item(&mut self, item: T, frequency: u32) {
    self.rand_range = RandRangeInclusive32::new(1..=self.rand_range.high() + frequency.max(1));
    self.items.push((frequency, item));
  }

  pub fn roll_with(&self, gen: &mut PCG32) -> T {
    let mut roll = self.rand_range.roll_with(gen);
    for item_ref in self.items.iter() {
      if roll <= item_ref.0 {
        return item_ref.1.clone();
      } else {
        roll -= item_ref.0;
      }
    }
    unreachable!("What the heck?");
  }
}
```

So we make a `FrequencyChart` and then stuff it with potential things and then
it rolls among them and gives us a clone back of the thing it picked.

```rust
    // Figure out what the chances of a random item dropping are.
    let depth_u32 = self.deepest_depth as u32;
    let mut item_frequencies = FrequencyChart::new(Item::PotionStrength, depth_u32);
    item_frequencies.push_item(Item::PotionHealth, depth_u32 + 20);
    item_frequencies.push_item(Item::BombBlast, depth_u32);
    item_frequencies.push_item(Item::BombIce, depth_u32.saturating_sub(10));
    item_frequencies.push_item(Item::Dagger(0), depth_u32 + 5);
    item_frequencies.push_item(Item::Warhammer(0), 2 * depth_u32);
    item_frequencies.push_item(Item::Fernweave(0), depth_u32 / 2 + 5);
    item_frequencies.push_item(Item::LobsterMail(0), depth_u32);
    item_frequencies.push_item(Item::CrystalPlate(0), depth_u32);

    // Place the Items
    for _ in 0..GAME_DIMENSIONS {
      let item_spot = self.pick_random_floor(self.deepest_depth);
      let new_item = item_frequencies.roll_with(&mut self.gen);
      self.item_locations.entry(item_spot).or_insert(Vec::new()).push(new_item);
    }
```

All those chances are just whatever came into my head, no particular game
balance there, just that I wanted them to shift as you went deeper.

Now we should be able to see items on the ground and in our inventory, even
though we can't use them yet. Let's turn it on. Oh no, we can't, because there's
a panic.

```
thread 'main' panicked at 'RandRangeInclusive32 must go from low to high, got 1 ..= 1', src\prng.rs:117:5
note: Run with `RUST_BACKTRACE=1` for a backtrace.
error: process didn't exit successfully: `target\debug\kasidin.exe` (exit code: 101)
```

Hmm. Hmm, hmm, hmmmmmm.

```rust
  pub fn new(range_incl: RangeInclusive<u32>) -> Self {
    let (low, high) = range_incl.into_inner();
    assert!(low < high, "RandRangeInclusive32 must go from low to high, got {} ..= {}", low, high);
    let base = low;
    let width = (high - low) + 1;
    debug_assert!(width > 0);
    let width_count = ::std::u32::MAX / width;
    let reject = (width_count * width) - 1;
    RandRangeInclusive32 { base, width, reject }
  }
```

Is our assert wrong? I think the assert is wrong. Let's just walk through it by
hand starting with low=1 and high=1.

```
low=1, high=1
low=1, high=1, so base=1
low=1, high=1, base=1, so width=1
width > 0 passes
low=1, high=1, base=1, width=1, so width_count=MAX
low=1, high=1, base=1, width=1, width_count=MAX, so reject=MAX-1

then later, when we need to use convert
base=1, width=1, reject=MAX-1;
roll=?
if roll > reject will trigger 1 time in MAX
else 1 + (roll % 1) all other times
```

So we should be all good. Probably. I'll change the assert to `low<=high`. We
can also re-run all our tests, and they pass. So let's proceed and assume that
we were just too defensive before.

```
thread 'main' panicked at 'internal error: entered unreachable code: What the heck?', src\prng.rs:103:5
note: Run with `RUST_BACKTRACE=1` for a backtrace.
error: process didn't exit successfully: `target\debug\kasidin.exe` (exit code: 101)
```

Ah ha, we were quite probably not too defensive before! Let's make `roll_with`
more particular about what it puts out for a moment.

```rust
  pub fn roll_with(&self, gen: &mut PCG32) -> u32 {
    loop {
      if let Some(output) = self.convert(gen.next_u32()) {
        debug_assert!(output >= self.low());
        debug_assert!(output <= self.high());
        return output;
      }
    }
  }
```

And we get the same panic. So `roll_with` was not at fault. Let's review what we
thought was unreachable:

```rust
  pub fn roll_with(&self, gen: &mut PCG32) -> T {
    let mut roll = self.rand_range.roll_with(gen);
    for item_ref in self.items.iter() {
      if roll <= item_ref.0 {
        return item_ref.1.clone();
      } else {
        roll -= item_ref.0;
      }
    }
    unreachable!("What the heck?");
  }
```

Hmm, we can add a big doctest to the front of this:

```rust
  /// ```rust
  /// use roguelike_tutorial_2018::*;
  /// let mut gen = &mut PCG32::new(u64_from_time());
  /// let mut chart = FrequencyChart::new('a', 1);
  /// // with just item we will ALWAYS see that item
  /// for _ in 0 .. 100 {
  ///   assert_eq!(chart.roll_with(gen), 'a');
  /// }
  /// chart.push_item('b', 1);
  /// let mut totals = [0i32,0];
  /// for _ in 0 .. 1000 {
  ///   match chart.roll_with(gen) {
  ///     'a' => totals[0] += 1,
  ///     'b' => totals[1] += 1,
  ///     z => panic!("impossible output {}",z),
  ///   }
  /// }
  /// assert!((totals[0] - totals[1]).abs() < 100);
  /// ```
```

But no use, the test passes without revealing anything.

So I think the problem is when we have more than one item. Let's look at how
`push_item` works again.

```rust
  pub fn push_item(&mut self, item: T, frequency: u32) {
    self.rand_range = RandRangeInclusive32::new(1..=self.rand_range.high() + frequency.max(1));
    self.items.push((frequency, item));
  }
```

Ah, of course, I'm a dummy. Do you see it too?

I'm making frequency be a minimum of 1 in some places but not in other places.
How obvious.

```rust
  pub fn new(item: T, frequency: u32) -> Self {
    let frequency = frequency.max(1);
    Self {
      rand_range: RandRangeInclusive32::new(1..=frequency),
      items: vec![(frequency, item)],
    }
  }

  pub fn push_item(&mut self, item: T, frequency: u32) {
    let frequency = frequency.max(1);
    self.rand_range = RandRangeInclusive32::new(1..=self.rand_range.high() + frequency);
    self.items.push((frequency, item));
  }
```

Okay _this code should work_, I think.

![items-on-the-ground](https://github.com/Lokathor/roguelike-tutorial-2018/blob/master/screenshots/week07-01.png)

![items-in-inventory](https://github.com/Lokathor/roguelike-tutorial-2018/blob/master/screenshots/week07-02.png)

Alright, so we're rocking along so far.

How do we want to equip an item? Well... I guess we'll just make a separate
inventory of "things you've equipped". Then we shuffle things back and forth as
you go along.

But... what are the effects of equipment? Well, I guess weapons add to your
damage_step when you put them on, and for armor we'll add a new value to
creatures.

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct Creature {
  pub icon: u8,
  pub color: u32,
  pub is_the_player: bool,
  pub id: CreatureID,
  pub hit_points: i32,
  pub damage_step: i32,
  pub armor: i32,
  pub inventory: Vec<Item>,
  pub equipment: Vec<Item>,
}
```

Hmm, but this way you've gotta sort out where in the `equipment` is the current
weapon when you go to replace it and stuff. That's no good. Let's try again.

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct Creature {
  pub icon: u8,
  pub color: u32,
  pub is_the_player: bool,
  pub id: CreatureID,
  pub hit_points: i32,
  pub damage_step: i32,
  pub armor: i32,
  pub inventory: Vec<Item>,
  pub equipped_weapon: Option<Item>,
  pub equipped_armor: Option<Item>,
}
```

This saves on some heap allocation, and it even takes less space than the `Vec`
style. Rust knows how to pack data in pretty well, so the size of an `Item` and
an `Option<Item>` are both going to be 2 bytes (one byte for the potential `i8`,
the other for the tag info, and the `None` possibility basically becomes another
tag when you wrap `Option` around `Item`). The size of an empty `Vec` is
`3*size_of::<usize>()` all on its own (pointer, len, capacity), and then putting
two items on top of that would be even more space used up. So that's nice.

Now we just apply a big update to `use_item`.

```rust
  pub fn use_item(&mut self, item_letter: char) -> UseItemResult {
    let player_mut = self.creature_list.iter_mut().find(|creature_ref| creature_ref.is_the_player).unwrap();
    let potential_item_to_use = {
      let mut cataloged_inventory = BTreeMap::new();
      for item_ref in player_mut.inventory.iter() {
        *cataloged_inventory.entry(item_ref).or_insert(0) += 1;
      }
      let letter_index = item_letter as u8 - 'a' as u8;
      cataloged_inventory.into_iter().nth(letter_index as usize).map(|(&item, _count)| item)
    };
    match potential_item_to_use {
      Some(item) => {
        match item {
          Item::BombBlast | Item::BombIce => return UseItemResult::ItemNeedsTarget,
          Item::PotionHealth | Item::PotionStrength => {
            apply_potion(&item, player_mut, &mut self.gen);
          }
          Item::CrystalPlate(_) | Item::Fernweave(_) | Item::LobsterMail(_) => {
            player_mut.equipped_armor.take().map(|old_armor| player_mut.inventory.push(old_armor));
            player_mut.equipped_armor = Some(item);
            player_mut.armor = item.armor_value();
          }
          Item::Dagger(_) | Item::Warhammer(_) => {
            player_mut.equipped_weapon.take().map(|old_weapon| {
              player_mut.damage_step -= old_weapon.damage_step();
              player_mut.inventory.push(old_weapon);
            });
            player_mut.equipped_weapon = Some(item);
            player_mut.damage_step += item.damage_step();
          }
        }
        for i in 0..player_mut.inventory.len() {
          if player_mut.inventory[i] == item {
            player_mut.inventory.remove(i);
            break;
          }
        }
        self.run_world_turn();
        UseItemResult::ItemUsed
      }
      None => UseItemResult::NoSuchItem,
    }
  }
```

Which also gives us some new helpers on `Item`,

```rust
  fn armor_value(self) -> i32 {
    match self {
      Item::Fernweave(x) => 2 + x as i32,
      Item::LobsterMail(x) => 5 + x as i32,
      Item::CrystalPlate(x) => 7 + x as i32,
      _ => 0,
    }
  }

  fn damage_step(self) -> i32 {
    match self {
      Item::Dagger(x) => 2 + x as i32,
      Item::Warhammer(x) => 7 + x as i32,
      _ => 0,
    }
  }
```

and we can _throw out_ the old `is_potion` method.

Of course, if the player can't see the changes that's not really any good. So
we'll just stick that into the status bar for now.

```rust
  let player_ref = game.creature_list.iter().find(|creature_ref| creature_ref.is_the_player).unwrap();
  write!(
    status_line_u8_slice_mut,
    "HP: {}, Enemies: {}, Z:{}, Damage Step: {}, Armor: {}",
    player_ref.hit_points,
    game.creature_list.len() - 1,
    game.player_location.z,
    player_ref.damage_step,
    player_ref.armor
  ).ok();
```

![now-with-status](https://github.com/Lokathor/roguelike-tutorial-2018/blob/master/screenshots/week07-03.png)

And we're good!

One note though: remember how we made the kestrels have the depth added to their
hit points last time? Well, we need to fix that, because the depth is usually
negative!

```rust
  fn new_kestrel(cid: usize, depth: i32) -> Self {
    let mut out = Self::new(cid, b'k', KESTREL_RED);
    out.hit_points = 8 + depth.abs();
    out.damage_step = 3;
    out
  }
```

Okay, now we're really good.
