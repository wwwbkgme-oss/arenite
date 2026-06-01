// arenite-core::items — Starbound-inspired item and inventory system
//
// Architecture mirrors Starbound's StarItem / StarItemDescriptor /
// StarItemBag / StarPlayerInventory with Rust idioms:
//
//  Item           — static definition in the item registry (like StarItem)
//  ItemStack      — (item_id, count) pair stored in an inventory slot
//  Inventory      — a fixed-capacity array of Option<ItemStack> slots
//  PlayerInventory — hotbar + main bag + armor slots (StarPlayerInventory)
//  ItemRegistry   — Registry<Item> loaded at startup
//
// Starbound JSON item fields mapped to Rust:
//   "shortdescription" → display_name
//   "description"      → description
//   "rarity"           → rarity: ItemRarity
//   "category"         → category: ItemCategory
//   "maxStack"         → max_stack
//   "price"            → price
//   "primaryAbility"   → dig_radius / damage / mining_speed fields

use crate::registry::Registry;
use serde::{Deserialize, Serialize};

// ── Rarity ────────────────────────────────────────────────────────────────────

/// Item rarity tier (Starbound / Terraria rarity system).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ItemRarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

impl ItemRarity {
    /// RGB colour used to tint rarity labels in the HUD (Terraria-style).
    pub fn colour(self) -> [u8; 3] {
        match self {
            Self::Common => [200, 200, 200],
            Self::Uncommon => [100, 230, 100],
            Self::Rare => [80, 130, 255],
            Self::Epic => [200, 80, 255],
            Self::Legendary => [255, 200, 0],
        }
    }
}

// ── Category ──────────────────────────────────────────────────────────────────

/// Broad functional category of an item (Starbound category field).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemCategory {
    /// Shovel/pickaxe/drill — removes tiles.
    Tool,
    /// Melee or ranged weapon.
    Weapon,
    /// Raw material or crafting ingredient.
    Material,
    /// Food, potion, or other single-use item.
    Consumable,
    /// Wearable armour or accessory.
    Armor,
    /// Paints a specific material pixel on use (maps to a material registry key).
    TilePlacer,
    /// Miscellaneous / quest / currency.
    Misc,
}

// ── Item definition ───────────────────────────────────────────────────────────

/// Static definition of an item type stored in the global `ItemRegistry`.
///
/// Corresponds to Starbound's `StarItem` C++ class and its JSON asset files.
#[derive(Debug, Clone)]
pub struct Item {
    // ── Identity ──────────────────────────────────────────────────────────
    /// Human-readable name shown in HUD/inventory ("Iron Pickaxe").
    pub display_name: String,
    /// Flavour text shown in tooltip ("A sturdy pickaxe for tough stone.").
    pub description: String,
    /// Rarity tier affecting colour and drop weight.
    pub rarity: ItemRarity,
    /// Functional category.
    pub category: ItemCategory,
    /// Maximum stack size (1 for tools/weapons, up to 999 for materials).
    pub max_stack: u32,
    /// Base shop buy-price in coins.
    pub price: u32,

    // ── Tool / weapon stats ───────────────────────────────────────────────
    /// Paint/dig radius in pixels (TilePlacer / Tool).
    pub dig_radius: i32,
    /// Damage per hit (Weapon).
    pub damage: u32,
    /// Mining speed multiplier: 1.0 = normal, 2.0 = 2× faster (Tool).
    pub mining_speed: f32,

    // ── Tile-placer linkage ───────────────────────────────────────────────
    /// If set, left-click paints this material key (e.g. `"sand"`, `"water"`).
    /// Only meaningful when `category == TilePlacer`.
    pub material_key: Option<String>,

    // ── Consumable stats ──────────────────────────────────────────────────
    /// HP restored on use (Consumable).
    pub heal_amount: u32,
}

impl Item {
    pub fn builder(name: impl Into<String>) -> ItemBuilder {
        ItemBuilder {
            display_name: name.into(),
            description: String::new(),
            rarity: ItemRarity::Common,
            category: ItemCategory::Misc,
            max_stack: 1,
            price: 0,
            dig_radius: 2,
            damage: 0,
            mining_speed: 1.0,
            material_key: None,
            heal_amount: 0,
        }
    }
}

// ── Builder ───────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct ItemBuilder {
    display_name: String,
    description: String,
    rarity: ItemRarity,
    category: ItemCategory,
    max_stack: u32,
    price: u32,
    dig_radius: i32,
    damage: u32,
    mining_speed: f32,
    material_key: Option<String>,
    heal_amount: u32,
}

impl ItemBuilder {
    pub fn desc(mut self, d: impl Into<String>) -> Self {
        self.description = d.into();
        self
    }
    pub fn rarity(mut self, r: ItemRarity) -> Self {
        self.rarity = r;
        self
    }
    pub fn category(mut self, c: ItemCategory) -> Self {
        self.category = c;
        self
    }
    pub fn stack(mut self, n: u32) -> Self {
        self.max_stack = n;
        self
    }
    pub fn price(mut self, p: u32) -> Self {
        self.price = p;
        self
    }
    pub fn dig_radius(mut self, r: i32) -> Self {
        self.dig_radius = r;
        self
    }
    pub fn damage(mut self, d: u32) -> Self {
        self.damage = d;
        self
    }
    pub fn mining_speed(mut self, s: f32) -> Self {
        self.mining_speed = s;
        self
    }
    pub fn material(mut self, k: impl Into<String>) -> Self {
        self.material_key = Some(k.into());
        self
    }
    pub fn heal(mut self, h: u32) -> Self {
        self.heal_amount = h;
        self
    }

    pub fn build(self) -> Item {
        Item {
            display_name: self.display_name,
            description: self.description,
            rarity: self.rarity,
            category: self.category,
            max_stack: self.max_stack,
            price: self.price,
            dig_radius: self.dig_radius,
            damage: self.damage,
            mining_speed: self.mining_speed,
            material_key: self.material_key,
            heal_amount: self.heal_amount,
        }
    }
}

// ── Registry ──────────────────────────────────────────────────────────────────

pub type ItemRegistry = Registry<Item>;

/// Build the default starter item set.
///
/// Includes tile-placers for all 26 materials, a progression of pickaxes
/// (wooden → iron → gold → titanium), matching swords, and some consumables.
/// Inspired by the item tiers in Starbound and Terraria.
pub fn default_item_registry() -> ItemRegistry {
    use ItemCategory::*;
    use ItemRarity::*;

    let mut r = ItemRegistry::new();

    // ── Tile placers (one per registered material) ────────────────────────
    macro_rules! placer {
        ($key:expr, $name:expr, $stack:expr) => {
            r.register(
                $key,
                Item::builder($name)
                    .category(TilePlacer)
                    .material($key)
                    .stack($stack)
                    .price(1)
                    .build(),
            )
        };
    }

    placer!("dirt", "Dirt", 999);
    placer!("stone", "Stone", 999);
    placer!("sand", "Sand", 999);
    placer!("gravel", "Gravel", 999);
    placer!("water", "Water Bucket", 64);
    placer!("lava", "Lava Bucket", 16);
    placer!("steam", "Steam Flask", 16);
    placer!("smoke", "Smoke Bomb", 16);
    placer!("fire", "Fire Bomb", 16);
    placer!("grass", "Grass Block", 999);
    placer!("snow", "Snow", 999);
    placer!("wood", "Wood Block", 999);
    placer!("gold_ore", "Gold Ore", 999);
    placer!("iron_ore", "Iron Ore", 999);
    placer!("clay", "Clay", 999);
    placer!("mud", "Mud", 999);
    placer!("obsidian", "Obsidian", 999);
    placer!("oil", "Oil Flask", 64);
    placer!("acid", "Acid Flask", 32);
    placer!("ice", "Ice Block", 999);
    placer!("crystal", "Crystal", 64);
    placer!("mushroom_block", "Mushroom Block", 999);
    placer!("copper_ore", "Copper Ore", 999);
    placer!("titanium_ore", "Titanium Ore", 999);
    placer!("diamond", "Diamond", 64);

    // ── Tools — pickaxes (Terraria / Starbound progression) ───────────────

    r.register(
        "wooden_pickaxe",
        Item::builder("Wooden Pickaxe")
            .desc("A basic pickaxe carved from wood.  Mines soft terrain.")
            .category(Tool)
            .rarity(Common)
            .price(50)
            .dig_radius(2)
            .mining_speed(0.7)
            .build(),
    );

    r.register(
        "iron_pickaxe",
        Item::builder("Iron Pickaxe")
            .desc("A sturdy iron pickaxe.  Mines most stone without issue.")
            .category(Tool)
            .rarity(Uncommon)
            .price(300)
            .dig_radius(3)
            .mining_speed(1.0)
            .build(),
    );

    r.register(
        "gold_pickaxe",
        Item::builder("Gold Pickaxe")
            .desc("Gleaming gold — faster but still not suitable for obsidian.")
            .category(Tool)
            .rarity(Rare)
            .price(800)
            .dig_radius(3)
            .mining_speed(1.4)
            .build(),
    );

    r.register(
        "titanium_pickaxe",
        Item::builder("Titanium Pickaxe")
            .desc("Space-age alloy.  Cuts through obsidian like butter.")
            .category(Tool)
            .rarity(Epic)
            .price(2500)
            .dig_radius(4)
            .mining_speed(2.0)
            .build(),
    );

    r.register(
        "diamond_drill",
        Item::builder("Diamond Drill")
            .desc("The pinnacle of mining technology — drills anything.")
            .category(Tool)
            .rarity(Legendary)
            .price(8000)
            .dig_radius(5)
            .mining_speed(3.5)
            .build(),
    );

    // ── Weapons — swords ─────────────────────────────────────────────────

    r.register(
        "wooden_sword",
        Item::builder("Wooden Sword")
            .desc("Better than nothing.  Barely.")
            .category(Weapon)
            .rarity(Common)
            .price(30)
            .damage(5)
            .build(),
    );

    r.register(
        "iron_sword",
        Item::builder("Iron Sword")
            .desc("A reliable iron blade.  Standard soldier issue.")
            .category(Weapon)
            .rarity(Uncommon)
            .price(250)
            .damage(15)
            .build(),
    );

    r.register(
        "gold_sword",
        Item::builder("Gold Sword")
            .desc("Gold is soft but this is surprisingly sharp.")
            .category(Weapon)
            .rarity(Rare)
            .price(700)
            .damage(22)
            .build(),
    );

    r.register(
        "titanium_sword",
        Item::builder("Titanium Sword")
            .desc("Cuts through most armour with ease.")
            .category(Weapon)
            .rarity(Epic)
            .price(2000)
            .damage(40)
            .build(),
    );

    r.register(
        "crystal_sword",
        Item::builder("Crystal Sword")
            .desc("Forged from deep crystal.  It hums faintly.")
            .category(Weapon)
            .rarity(Legendary)
            .price(6000)
            .damage(65)
            .build(),
    );

    // ── Consumables ───────────────────────────────────────────────────────

    r.register(
        "health_potion",
        Item::builder("Health Potion")
            .desc("Restores 50 HP.  Tastes faintly of copper.")
            .category(Consumable)
            .rarity(Common)
            .stack(20)
            .price(25)
            .heal(50)
            .build(),
    );

    r.register(
        "mega_health_potion",
        Item::builder("Mega Health Potion")
            .desc("Restores 200 HP.  Don't drink too fast.")
            .category(Consumable)
            .rarity(Rare)
            .stack(10)
            .price(200)
            .heal(200)
            .build(),
    );

    r
}

// ── ItemStack ─────────────────────────────────────────────────────────────────

/// A stack of items occupying a single inventory slot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemStack {
    /// Registry key into the `ItemRegistry`.
    pub item_key: String,
    /// Number of items in this stack.
    pub count: u32,
}

impl ItemStack {
    pub fn new(item_key: impl Into<String>, count: u32) -> Self {
        Self {
            item_key: item_key.into(),
            count,
        }
    }
}

// ── Inventory ─────────────────────────────────────────────────────────────────

/// A fixed-capacity array of item slots (Starbound `StarItemBag`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inventory {
    pub slots: Vec<Option<ItemStack>>,
}

impl Inventory {
    pub fn new(capacity: usize) -> Self {
        Self {
            slots: vec![None; capacity],
        }
    }

    pub fn capacity(&self) -> usize {
        self.slots.len()
    }

    /// Insert `stack` into the first empty slot.  Returns `false` if full.
    pub fn add(&mut self, stack: ItemStack) -> bool {
        // First try to merge into an existing stack of the same key.
        for slot in self.slots.iter_mut().flatten() {
            if slot.item_key == stack.item_key {
                slot.count += stack.count;
                return true;
            }
        }
        // Then find an empty slot.
        for slot in self.slots.iter_mut() {
            if slot.is_none() {
                *slot = Some(stack);
                return true;
            }
        }
        false
    }

    pub fn get(&self, index: usize) -> Option<&ItemStack> {
        self.slots.get(index)?.as_ref()
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut ItemStack> {
        self.slots.get_mut(index)?.as_mut()
    }

    pub fn remove(&mut self, index: usize, count: u32) -> bool {
        if let Some(Some(stack)) = self.slots.get_mut(index) {
            if stack.count >= count {
                stack.count -= count;
                if stack.count == 0 {
                    self.slots[index] = None;
                }
                return true;
            }
        }
        false
    }
}

// ── PlayerInventory ───────────────────────────────────────────────────────────

/// The player's full inventory (Starbound `StarPlayerInventory`).
///
/// Layout:
/// - `hotbar`:  8 slots mapped to keys 1–8 (the "action bar")
/// - `main`:    30-slot main bag
/// - `armor`:   4 slots: head / chest / legs / accessory
/// - `selected_hotbar`: which of the 8 hotbar slots is active
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInventory {
    pub hotbar: Inventory,
    pub main: Inventory,
    pub armor: Inventory,
    pub selected_hotbar: usize,
}

impl PlayerInventory {
    pub const HOTBAR_SIZE: usize = 8;
    pub const MAIN_SIZE: usize = 30;
    pub const ARMOR_SIZE: usize = 4;

    pub fn new() -> Self {
        Self {
            hotbar: Inventory::new(Self::HOTBAR_SIZE),
            main: Inventory::new(Self::MAIN_SIZE),
            armor: Inventory::new(Self::ARMOR_SIZE),
            selected_hotbar: 0,
        }
    }

    /// Build a starter inventory with common materials and a basic pickaxe/sword.
    pub fn starter() -> Self {
        let mut inv = Self::new();
        // Hotbar: commonly used materials and tools
        let hotbar_default: &[(&str, u32)] = &[
            ("wooden_pickaxe", 1),
            ("wooden_sword", 1),
            ("dirt", 100),
            ("stone", 100),
            ("sand", 50),
            ("water", 20),
            ("lava", 5),
            ("fire", 5),
        ];
        for (key, count) in hotbar_default {
            inv.hotbar.add(ItemStack::new(*key, *count));
        }
        // Main bag: extra building materials
        let main_default: &[(&str, u32)] = &[
            ("wood", 200),
            ("grass", 50),
            ("snow", 50),
            ("clay", 30),
            ("iron_ore", 10),
            ("health_potion", 5),
        ];
        for (key, count) in main_default {
            inv.main.add(ItemStack::new(*key, *count));
        }
        inv
    }

    /// Return the active hotbar slot.
    pub fn active_slot(&self) -> Option<&ItemStack> {
        self.hotbar.get(self.selected_hotbar)
    }

    /// Return the `material_key` for the active hotbar item, if it is a tile-placer.
    pub fn active_material_key<'a>(&'a self, registry: &'a ItemRegistry) -> Option<&'a str> {
        let stack = self.active_slot()?;
        let key = crate::id::StringId::from(stack.item_key.as_str());
        let item = registry.get_by_name(&key)?;
        item.material_key.as_deref()
    }

    pub fn select_slot(&mut self, slot: usize) {
        if slot < Self::HOTBAR_SIZE {
            self.selected_hotbar = slot;
        }
    }

    pub fn cycle_next(&mut self) {
        self.selected_hotbar = (self.selected_hotbar + 1) % Self::HOTBAR_SIZE;
    }

    pub fn cycle_prev(&mut self) {
        if self.selected_hotbar == 0 {
            self.selected_hotbar = Self::HOTBAR_SIZE - 1;
        } else {
            self.selected_hotbar -= 1;
        }
    }
}

impl Default for PlayerInventory {
    fn default() -> Self {
        Self::new()
    }
}
