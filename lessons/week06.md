# Week 06

## Part 10: Saving and Loading

So next thing to do is saving and loading.

This is one of the few places where we'll pull in a crate to do it.

Basically, saving and loading is easy: you take a struct, write it out one field
at a time into some file, and then to load it you later you read one field at a
time until you've got enough data to rebuild the struct. There's a whole
[std::io](https://doc.rust-lang.org/std/io/index.html) module full of file
reading and writing stuff. It's as good as any other file IO stuff really, but
doing manual file IO is just not really that fun to begin with. Also, we're
changing our game data _all the time_ as we develop. There's no way we want to
fiddle with updating the save/load code each time we adjust a field in some
struct somewhere.

This is where [serde](https://crates.io/crates/serde) comes in. It can do all
that saving work for us any time something has the `Serialize` trait, and it can
do all the loading work with anything that has the `Deserialize` trait. The most
important part of this is that it also has stuff to _derive those traits for
you_. It's pretty wiz, chummer.

I have _literally_ never used the `serde` crate before, I've actually just heard
about it and thought "oh i'll use that someday probably", well this is that
someday. So I'm documenting all this exactly as I go through it for the first
time. The readme on their crates.io page says that all we gotta do is follow the
steps in [their guide about it](https://serde.rs/derive.html).

* Add serde = "1.0" as a dependency in Cargo.toml.
* Add serde_derive = "1.0" as a dependency in Cargo.toml.
* Ensure that all other Serde-based dependencies (for example serde_json) are on a version that is compatible with serde 1.0.

Easy as pie. `serde` is the core crate, `serde_derive` offers the special derive
ability, and then there's helper crates that offer different formats. Did I
mention that the `serde` crate is as format-agnostic as possible, so we'll be
able to pick from [like 10 different formats](https://serde.rs/#data-formats)?
Looking at that list, we don't really want to encourage players to edit their
save file stuff, so we won't bother with JSON. I don't know if other formats are
built into `serde` already, but I like the sound of the `bincode` and `cbor`
formats just because it sounds like they take up less space. The
[bincode](https://crates.io/crates/bincode) crate has an example on the front
page it seems, and it uses serde 1.0.63. The latest serde is 1.0.70, so we
should be all good.

```toml
[dependencies]
dwarf-term = "0.1"
serde = "1.0"
serde_derive = "1.0"
bincode = "1.0"
```

* If you have a main.rs, add #[macro_use] extern crate serde_derive there.

And we're using `bincode` too, so I think in our kasidin.rs file we put this:

```rust
extern crate serde;

#[macro_use]
extern crate serde_derive;

extern crate bincode;
```

* If you have a lib.rs, add #[macro_use] extern crate serde_derive there.

The lib.rs doesn't need to know about the `bincode` format, that's the point, so
we don't add that part there, just the `serde` and `serde_derive` macros.

* Use #[derive(Serialize)] on structs and enums that you want to serialize.
* Use #[derive(Deserialize)] on structs and enums that you want to deserialize.

Easy. First we add it to the `GameWorld` type and then it should complain if
other types inside can't be figured out. Looks like Creature, CreatureID, Item,
Terrain, PCG32, and Location will all need it. Eventually it all builds.

```
D:\dev\roguelike-tutorial-2018>cargo build
   Compiling roguelike-tutorial-2018 v0.5.0-pre (file:///D:/dev/roguelike-tutorial-2018)
warning: unused `#[macro_use]` import
  --> src\bin\kasidin.rs:14:1
   |
14 | #[macro_use]
   | ^^^^^^^^^^^^
   |
   = note: #[warn(unused_imports)] on by default

    Finished dev [unoptimized + debuginfo] target(s) in 5.30s
```

Of course, we're missing something.

Do you remember what it is?

Of course, that global that I told you would be totally safe, that's no longer
so safe. We gotta move it into being part of `GameWorld`. Well, we'll just
delete the `AtomicUsize` thing and then make `next_creature_id` be a `usize`
field in the `GameWorld`. We gotta move some code around to make it all match up
now, but that's no big thing.

Okay, so the _whole game_ is now wrapped up in our `GameWorld` struct. Let's,
uh, save it I guess? We don't really have a title screen where you can pick to
continue a game or start a new one. We'll just say that you save the game with
F5 and you can load the game with F6.

```rust
    for key in pending_keys.drain(..) {
      match display_mode {
        DisplayMode::Game => match key {
          VirtualKeyCode::Up => game.move_player(Location { x: 0, y: 1 }),
          VirtualKeyCode::Down => game.move_player(Location { x: 0, y: -1 }),
          VirtualKeyCode::Left => game.move_player(Location { x: -1, y: 0 }),
          VirtualKeyCode::Right => game.move_player(Location { x: 1, y: 0 }),
          VirtualKeyCode::I => display_mode = DisplayMode::Inventory,
          VirtualKeyCode::F5 => {
            save_game(&game).ok();
          }
          VirtualKeyCode::F6 => {
            load_game(&mut game).ok();
          }
          _ => {}
        },
```

And of course our outlines

```rust
fn save_game(game: &GameWorld) -> std::io::Result<()> {
  unimplemented!()
}

fn load_game(game: &mut GameWorld) -> std::io::Result<()> {
  unimplemented!()
}
```

For save game, I guess first we open a `File` with the
[create](https://doc.rust-lang.org/std/fs/struct.File.html#method.create)
method. Then, according to the `bincode` example, we use its `serialize`
function, and then... I guess we can simply write all the bytes into that file
and we're done.

```rust
fn save_game(game: &GameWorld) -> std::io::Result<()> {
  let mut f = std::fs::File::create("kasidin.save")?;
  let encoded: Vec<u8> = bincode::serialize(&game).map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "Couldn't serialize the game!"))?;
  f.write_all(&encoded)
}
```

The little `?` operator there, almost so small you can miss it, does an "unwrap
or early return the error" sort of thing. Rust doesn't have an exception hook
system like C++ and Java and friends, so you can either `panic` or encode your
possible failure into the return type. The `?` operator is the best way to
handle all those intermediate "keep going only if we're good so far" steps.

To load up the game we just do the same thing in reverse order basically.

```rust
fn load_game(game: &mut GameWorld) -> std::io::Result<()> {
  let mut f = std::fs::File::open("kasidin.save")?;
  let mut file_bytes: Vec<u8> = vec![];
  f.read_to_end(&mut file_bytes)?;
  let decoded: GameWorld =
    bincode::deserialize(&file_bytes).map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "Couldn't deserialize the game!"))?;
  *game = decoded;
  Ok(())
}
```

And we're done. That was it.

## Part 11: Deeper Depths

Next part is that we want a way to add deeper depths to the game. For now we'll
just make it infinitely deep. Naturally, to do that we have to only generate
more depths as they're actually required.

Let's start by adding a `z` element to `Location` and seeing how much breaks.

```
error: aborting due to 19 previous errors
```

Hey that's not bad! A lot of this is fairly obvious. To do `add` with `Location`
values you need to add the `z` coordinates too now, stuff like that.

The `LocationNeighborsIter` type will stick to only giving neighbors for the 4
cardinal directions for now, since going between z-levels is usually considered
to be a "bigger deal" than going horizontally.

`GameWorld::pick_random_floor` needs to know what `z` to use now. I don't think
we want to have it picking a random floor _anywhere_ at all, so we'll supply the
`z` in that case.

For FOV stuff, like bomb blasts or actual FOV, we can just use the `z` of
whoever triggered the FOV usage.

Okay so we've got no compile errors, now we need to move the player down a layer
when they type `'>'`, and up a layer for `'<'`, but only when on an appropriate
stairs tile. Sounds like we need two new types of terrain.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Terrain {
  Wall,
  Floor,
  Ice,
  StairsDown,
  StairsUp,
}
```

And, of course, we need to fiddle all the spots that consider terrain to handle
these two new terrain types. Not a big deal there.

Now we need to add at least 1 down stairs. That's easy, we just pick a random
floor after we make each floor and make it a down stairs.

```rust
    let stairs_place = out.pick_random_floor(out.deepest_depth);
    out.terrain.insert(stairs_place, Terrain::StairsDown);
```

And finally we add the code in the binary to go down and up.

Hmm, what key is the LessThan character? I don't really see it in [the
VirtualKeyCode](https://docs.rs/glutin/0.16.0/glutin/enum.VirtualKeyCode.html)
declaration. Let's find out!

```rust
      match display_mode {
        DisplayMode::Game => match key {
          VirtualKeyCode::Up => game.move_player(Location { x: 0, y: 1 }),
          VirtualKeyCode::Down => game.move_player(Location { x: 0, y: -1 }),
          VirtualKeyCode::Left => game.move_player(Location { x: -1, y: 0 }),
          VirtualKeyCode::Right => game.move_player(Location { x: 1, y: 0 }),
          VirtualKeyCode::I => display_mode = DisplayMode::Inventory,
          VirtualKeyCode::F5 => {
            save_game(&game).ok();
          }
          VirtualKeyCode::F6 => {
            load_game(&mut game).ok();
          }

          o => println!("{:?}", o),
        },
```

then

```
D:\dev\roguelike-tutorial-2018>cargo run
    Finished dev [unoptimized + debuginfo] target(s) in 0.07s
     Running `target\debug\kasidin.exe`
RShift
Period
```

Okay, so we have to track the shift state as well, then check for Shift+Period.

```rust
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
```

And now... basically all of our key processing requires that we check for shift
= false all the time. Oh well. Now we can finally read the right keys to use
stairs. We'll make `change_floor` be a different method from normal so that we
can potentially load and unload things as necessary during floor changes later
on. It will verify that everything is in place and then call to `move_player`.

```rust
          (VirtualKeyCode::Period, true) => game.change_floor(-1),
          (VirtualKeyCode::Comma, true) => game.change_floor(1),
```

And we'll start this off simple.

```rust
  pub fn change_floor(&mut self, floor_delta: i32) {
    let player_terrain = self.terrain[&self.player_location];
    match (player_terrain, floor_delta) {
      (Terrain::StairsDown, -1) => {
        println!("down!");
      }
      (Terrain::StairsUp, 1) => {
        println!("up!");
      }
      _ => {}
    }
  }
```

Now we just turn on the game and find some StairsDown to test it on. One
problem, the map is big and twisty and it's hard to even find the stairs at all.
Now that we'll be going down, let's make it not as wide and tall. 50x50 should
be fine, with less creatures and items to match. Now we can finally find some
stairs!

![stairs](https://github.com/Lokathor/roguelike-tutorial-2018/blob/master/screenshots/week06-01.png)

If the player goes down past their deepest floor so far, we need to generate a
new floor. Let's package all the previous floor generation code up into a method
on GameWorld.

```rust
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
    out.add_z_layer();

    // Place the Player
    let player_start = out.pick_random_floor(out.deepest_depth);
    let player_id = player.id.0;
    out.creature_list.push(player);
    out.creature_locations.insert(player_start, CreatureID(player_id));
    out.player_location = player_start;

    out
  }

  pub fn add_z_layer(&mut self) {
    self.deepest_depth += 1;

    // Place the Terrain
    let caves = make_cellular_caves(GAME_DIMENSIONS, GAME_DIMENSIONS, &mut self.gen);
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
```

Okay... except now we're not assured that we'll have StairsUp in the place that
we go down. We should check that the new z layer has a floor tile at the x,y
we're using, if not, we'll make another z layer until we find a good layer. With
only a single location as the constraint it shouldn't be a problem to just make
a new layers over and over. For the first layer, we don't need any constraint at
all, so we'll make that an `Option<Location>` argument I guess.

```rust
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
```

So now when we change floor, we check if we need to add a z-layer, and then we
pass along control to move_player.

```rust
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
```

But when we play this for a bit to find some stairs and then go down the
stairs... we get a panic!

```
thread 'main' panicked at 'couldn't find a path', libcore\option.rs:960:5
note: Run with `RUST_BACKTRACE=1` for a backtrace.
error: process didn't exit successfully: `target\debug\kasidin.exe` (exit code: 101)
```

That's because I was a little too careless in doing the FOV code for monsters
when I updated things to run in 3d. They should use _their own_ Z value for FOV,
not always the players. That much is obvious I guess. Why the panic
specifically? Well since pathing can't go between floors (because of the way we
setup out Neighbors iterator), it'll panic if they're looking at the wrong
floor, see the player, and then try to path from where they really are to the
player (because it's assumed that if the player is seen then a path can be
found).

Anyway, that's it. We can add a little indicator to the player's display to tell
them what Z they're at, but that's all we need to do for this week. It was a
pretty easy time I'd say.
