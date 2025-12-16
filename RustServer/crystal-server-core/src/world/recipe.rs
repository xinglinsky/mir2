use std::fs;
use std::io;
use std::path::Path;

use crate::world::content;

use crystal_shared_proto::io::{write_i32_le, write_u32_le};
use crystal_shared_proto::item_types::{ItemInfoData, UserItemData};
use crate::item::create_fresh_user_item;

#[derive(Clone, Debug)]
pub struct RecipeItemRequirement {
    pub item_index: i32,
    pub count: u16,
    pub current_dura: Option<u16>,
}

#[derive(Clone, Debug)]
pub struct RecipeInfo {
    pub item_index: i32,
    pub amount: u16,
    pub chance: u8,
    pub gold: u32,
    pub tools: Vec<RecipeItemRequirement>,
    pub ingredients: Vec<RecipeItemRequirement>,
    pub required_flag: Vec<i32>,
    pub required_level: Option<u16>,
    pub required_quest: Vec<i32>,
    pub required_class: Vec<i32>,
    pub required_gender: Option<i32>,
}

impl RecipeInfo {
    pub fn matches_product(&self, item_index: i32) -> bool {
        self.item_index == item_index
    }

    pub fn encode_client_recipe_bytes(
        &self,
        item_infos: &[ItemInfoData],
        recipe_unique_id: u64,
    ) -> io::Result<Vec<u8>> {
        let mut buf = Vec::new();

        let product_info = item_infos
            .iter()
            .find(|i| i.index == self.item_index)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("unknown recipe product item_index {}", self.item_index),
                )
            })?;

        let mut product = create_fresh_user_item(product_info, recipe_unique_id, self.amount);
        product.is_shop_item = true;

        write_u32_le(&mut buf, self.gold)?;
        buf.push(self.chance);

        let product_bytes = product.encode_to_bytes()?;
        buf.extend_from_slice(&product_bytes);

        let mut tools_encoded: Vec<UserItemData> = Vec::new();
        for req in &self.tools {
            if let Some(info) = item_infos.iter().find(|i| i.index == req.item_index) {
                let mut item = create_fresh_user_item(info, 0, req.count.max(1));
                item.is_shop_item = true;
                tools_encoded.push(item);
            }
        }

        write_i32_le(&mut buf, tools_encoded.len() as i32)?;
        for tool in tools_encoded {
            let bytes = tool.encode_to_bytes()?;
            buf.extend_from_slice(&bytes);
        }

        let mut ingredients_encoded: Vec<UserItemData> = Vec::new();
        for req in &self.ingredients {
            if let Some(info) = item_infos.iter().find(|i| i.index == req.item_index) {
                let mut item = create_fresh_user_item(info, 0, req.count);
                item.is_shop_item = true;
                if let Some(dura) = req.current_dura {
                    let d = dura.min(item.max_dura);
                    item.current_dura = d;
                }
                ingredients_encoded.push(item);
            }
        }

        write_i32_le(&mut buf, ingredients_encoded.len() as i32)?;
        for ing in ingredients_encoded {
            let bytes = ing.encode_to_bytes()?;
            buf.extend_from_slice(&bytes);
        }

        Ok(buf)
    }
}

pub fn load_recipes_from_dir<P: AsRef<Path>>(
    root: P,
    item_infos: &[ItemInfoData],
) -> io::Result<Vec<RecipeInfo>> {
    let root = root.as_ref();
    let mut result = Vec::new();

    let mut recipe_paths: Vec<std::path::PathBuf> = Vec::new();

    match fs::read_dir(root) {
        Ok(entries) => {
            for entry in entries {
                let entry = match entry {
                    Ok(e) => e,
                    Err(_) => continue,
                };
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                match path.extension().and_then(|s| s.to_str()) {
                    Some(ext) if ext.eq_ignore_ascii_case("txt") => {
                        recipe_paths.push(path);
                    }
                    _ => continue,
                }
            }
        }
        Err(e) => {
            // If the directory doesn't exist (common when we delete Envir/ after enabling
            // content.pack), try enumerating the recipe files from the pack manifest.
            if e.kind() != io::ErrorKind::NotFound || !content::has_content_pack() {
                return Err(e);
            }

            tracing::debug!(
                "[recipe] Directory {} not found; enumerating recipes from content pack",
                root.display()
            );

            let prefix = root.to_string_lossy().replace('\\', "/");
            let paths = content::list_pack_paths_with_prefix(&prefix);
            for p in paths {
                if !p.to_ascii_lowercase().ends_with(".txt") {
                    continue;
                }
                recipe_paths.push(std::path::PathBuf::from(p));
            }
        }
    }

    recipe_paths.sort_by(|a, b| a.to_string_lossy().to_ascii_lowercase().cmp(&b.to_string_lossy().to_ascii_lowercase()));

    for path in recipe_paths {
        let name = match path.file_stem().and_then(|s| s.to_str()) {
            Some(n) if !n.is_empty() => n,
            _ => continue,
        };

        let product_info = match find_item_by_name(item_infos, name) {
            Some(info) => info,
            None => {
                eprintln!(
                    "[recipe] Could not find ItemInfo for recipe product '{}' from {}",
                    name,
                    path.display()
                );
                continue;
            }
        };

        let text = match content::read_to_string(&path) {
            Ok(t) => t,
            Err(e) => {
                eprintln!(
                    "[recipe] Failed to read recipe file {}: {}",
                    path.display(),
                    e
                );
                continue;
            }
        };

        let mut recipe = RecipeInfo {
            item_index: product_info.index,
            amount: 1,
            chance: 100,
            gold: 0,
            tools: Vec::new(),
            ingredients: Vec::new(),
            required_flag: Vec::new(),
            required_level: None,
            required_quest: Vec::new(),
            required_class: Vec::new(),
            required_gender: None,
        };

        if let Err(e) = parse_recipe_text(&text, &mut recipe, item_infos, name, &path) {
            eprintln!(
                "[recipe] Failed to parse recipe {} from {}: {}",
                name,
                path.display(),
                e
            );
            continue;
        }

        result.push(recipe);
    }

    Ok(result)
}

fn find_item_by_name<'a>(item_infos: &'a [ItemInfoData], name: &str) -> Option<&'a ItemInfoData> {
    item_infos
        .iter()
        .find(|i| i.name.eq_ignore_ascii_case(name))
}

fn parse_recipe_text(
    text: &str,
    recipe: &mut RecipeInfo,
    item_infos: &[ItemInfoData],
    recipe_name: &str,
    path: &Path,
) -> io::Result<()> {
    enum Section {
        Recipe,
        Tools,
        Ingredients,
        Criteria,
    }

    let mut mode = Section::Ingredients;

    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') && line.len() >= 2 {
            let name = &line[1..line.len() - 1];
            let lower = name.to_ascii_lowercase();
            mode = match lower.as_str() {
                "recipe" => Section::Recipe,
                "tools" => Section::Tools,
                "ingredients" => Section::Ingredients,
                "criteria" => Section::Criteria,
                _ => mode,
            };
            continue;
        }

        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }

        match mode {
            Section::Recipe => {
                if tokens.len() < 2 {
                    continue;
                }
                let key = tokens[0].to_ascii_lowercase();
                let value = tokens[1];
                match key.as_str() {
                    "amount" => {
                        if let Ok(v) = value.parse::<u16>() {
                            recipe.amount = v;
                        }
                    }
                    "chance" => {
                        if let Ok(v) = value.parse::<u8>() {
                            recipe.chance = if v > 100 { 100 } else { v };
                        }
                    }
                    "gold" => {
                        if let Ok(v) = value.parse::<u32>() {
                            recipe.gold = v;
                        }
                    }
                    _ => {}
                }
            }
            Section::Tools => {
                let name = tokens[0];
                let info = match find_item_by_name(item_infos, name) {
                    Some(i) => i,
                    None => {
                        eprintln!(
                            "[recipe] Could not find Tool '{}' in recipe '{}' ({})",
                            name,
                            recipe_name,
                            path.display()
                        );
                        continue;
                    }
                };
                let req = RecipeItemRequirement {
                    item_index: info.index,
                    count: 1,
                    current_dura: None,
                };
                recipe.tools.push(req);
            }
            Section::Ingredients => {
                let name = tokens[0];
                let info = match find_item_by_name(item_infos, name) {
                    Some(i) => i,
                    None => {
                        eprintln!(
                            "[recipe] Could not find Ingredient '{}' in recipe '{}' ({})",
                            name,
                            recipe_name,
                            path.display()
                        );
                        continue;
                    }
                };

                let mut count: u16 = 1;
                if tokens.len() >= 2 {
                    if let Ok(v) = tokens[1].parse::<u16>() {
                        count = v;
                    }
                }

                let mut current_dura: Option<u16> = None;
                if tokens.len() >= 3 {
                    if let Ok(v) = tokens[2].parse::<u16>() {
                        current_dura = Some(v);
                    }
                }

                if count as u32 > info.stack_size as u32 {
                    count = info.stack_size;
                }

                let req = RecipeItemRequirement {
                    item_index: info.index,
                    count,
                    current_dura,
                };
                recipe.ingredients.push(req);
            }
            Section::Criteria => {
                if tokens.len() < 2 {
                    continue;
                }
                let key = tokens[0].to_ascii_lowercase();
                let value = tokens[1];
                match key.as_str() {
                    "level" => {
                        if let Ok(v) = value.parse::<u16>() {
                            recipe.required_level = Some(v);
                        }
                    }
                    "flag" => {
                        if let Ok(v) = value.parse::<i32>() {
                            recipe.required_flag.push(v);
                        }
                    }
                    "quest" => {
                        if let Ok(v) = value.parse::<i32>() {
                            recipe.required_quest.push(v);
                        }
                    }
                    "class" => {
                        if let Ok(v) = value.parse::<i32>() {
                            recipe.required_class.push(v);
                        }
                    }
                    "gender" => {
                        if let Ok(v) = value.parse::<i32>() {
                            recipe.required_gender = Some(v);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}
