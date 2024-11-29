use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use farmfe_core::{
  context::CompilationContext,
  enhanced_magic_string::types::SourceMapOptions,
  error::CompilationError,
  module::{module_group::ModuleGroupId, Module, ModuleId},
  resource::resource_pot::{ResourcePot, ResourcePotType},
};

use crate::{
  generate::render_resource_pots::render_resource_pots_and_generate_resources, write_cache,
};

use super::diff_and_patch_module_graph::DiffResult;

mod generate_and_diff_resource_pots;

use generate_and_diff_resource_pots::generate_and_diff_resource_pots;

pub fn render_and_generate_update_resource(
  updated_module_ids: &Vec<ModuleId>,
  diff_result: &DiffResult,
  context: &Arc<CompilationContext>,
) -> farmfe_core::error::Result<(String, String)> {
  let mut immutable_update_resource_pot = ResourcePot::new(
    String::from("__IMMUTABLE_UPDATE_RESOURCE_POT__"),
    ResourcePotType::Js,
  );
  immutable_update_resource_pot.immutable = true;

  let mut mutable_update_resource_pot = ResourcePot::new(
    String::from("__MUTABLE_UPDATE_RESOURCE_POT__"),
    ResourcePotType::Js,
  );
  mutable_update_resource_pot.immutable = false;

  let module_graph = context.module_graph.read();

  for added in &diff_result.added_modules {
    let module = module_graph.module(added).unwrap();

    if module.external {
      continue;
    }

    if module.immutable {
      immutable_update_resource_pot.add_module(added.clone());
    } else {
      mutable_update_resource_pot.add_module(added.clone());
    }
  }

  for updated in updated_module_ids {
    let module = module_graph.module(updated).unwrap();

    if module.external {
      continue;
    }

    if module.immutable {
      immutable_update_resource_pot.add_module(updated.clone());
    } else {
      mutable_update_resource_pot.add_module(updated.clone());
    }
  }

  let gen_resource_pot_code =
    |resource_pot: &mut ResourcePot| -> farmfe_core::error::Result<String> {
      let res = context.plugin_driver.render_update_resource_pot(
        resource_pot,
        context,
        &Default::default(),
      )?;

      res.map_or(Ok("{}".to_string()), |r| Ok(r.content))
    };

  let immutable_update_resource = gen_resource_pot_code(&mut immutable_update_resource_pot)?;
  let mutable_update_resource = gen_resource_pot_code(&mut mutable_update_resource_pot)?;

  Ok((immutable_update_resource, mutable_update_resource))
}

pub fn regenerate_resources_for_affected_module_groups(
  affected_module_groups: HashSet<ModuleGroupId>,
  diff_result: DiffResult,
  updated_module_ids: &Vec<ModuleId>,
  removed_modules: &HashMap<ModuleId, Module>,
  context: &Arc<CompilationContext>,
) -> farmfe_core::error::Result<()> {
  // if there are deps changes, update execution order
  {
    let mut module_graph = context.module_graph.write();
    module_graph.update_execution_order_for_modules();
  }

  // skip diff resource pots if diff_result is empty
  let mut affected_resource_pots_ids = if diff_result.added_modules.is_empty()
    && diff_result.removed_modules.is_empty()
    && diff_result.deps_changes.is_empty()
  {
    vec![]
  } else {
    clear_resource_pot_of_modules_in_module_groups(&affected_module_groups, context);
    generate_and_diff_resource_pots(
      &affected_module_groups,
      &diff_result,
      updated_module_ids,
      removed_modules,
      context,
    )?
  };

  let mut resource_pot_map = context.resource_pot_map.write();
  // always rerender the updated module's resource pot
  let module_graph = context.module_graph.read();

  for updated_module_id in updated_module_ids {
    let module = module_graph.module(updated_module_id).unwrap();
    let resource_pot_id = module.resource_pot.as_ref().unwrap();

    if !affected_resource_pots_ids.contains(resource_pot_id) {
      affected_resource_pots_ids.push(resource_pot_id.clone());
    }

    // also remove the related resources, the resources will be regenerated later
    let mut resource_maps = context.resources_map.lock();
    let resource_pot = resource_pot_map.resource_pot_mut(resource_pot_id).unwrap();

    for resource in resource_pot.resources() {
      resource_maps.remove(resource);
    }

    resource_pot.clear_resources();
  }

  let mut resource_pots = resource_pot_map
    .resource_pots_mut()
    .into_iter()
    .filter(|rp| affected_resource_pots_ids.contains(&rp.id))
    .collect::<Vec<&mut ResourcePot>>();

  drop(module_graph);

  // call process_resource_pots hook
  context
    .plugin_driver
    .process_resource_pots(&mut resource_pots, context)?;

  render_resource_pots_and_generate_resources(resource_pots, context, &Default::default())?;

  if context.config.persistent_cache.enabled() {
    context
      .plugin_driver
      .write_plugin_cache(context)
      .unwrap_or_else(|err| {
        eprintln!("write plugin cache error: {err:?}");
      });

    write_cache(context.clone());
  }

  Ok(())
}

fn clear_resource_pot_of_modules_in_module_groups(
  module_group_id: &HashSet<ModuleGroupId>,
  context: &Arc<CompilationContext>,
) {
  for module_group_id in module_group_id {
    let mut module_graph = context.module_graph.write();
    let module_group_graph = context.module_group_graph.read();
    let module_group = module_group_graph.module_group(module_group_id).unwrap();

    for module_id in module_group.modules() {
      let module = module_graph.module_mut(module_id).unwrap();
      module.resource_pot = None;
    }
  }
}
