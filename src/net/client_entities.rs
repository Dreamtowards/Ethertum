use bevy::{ecs::system::EntityCommands, platform::collections::HashMap, prelude::*};

use crate::client::prelude::DespawnOnWorldUnload;

use super::EntityId;

/// Network IDs belong to the server; only Commands allocates local entity IDs.
#[derive(Resource, Default)]
pub(crate) struct ClientEntities(HashMap<EntityId, Entity>);

impl ClientEntities {
    pub fn get_or_spawn<'a>(&mut self, id: EntityId, commands: &'a mut Commands) -> EntityCommands<'a> {
        let entity = *self
            .0
            .entry(id)
            .or_insert_with(|| commands.spawn((Transform::default(), DespawnOnWorldUnload)).id());
        commands.entity(entity)
    }

    pub fn remove(&mut self, id: EntityId, commands: &mut Commands) {
        if let Some(entity) = self.0.remove(&id) {
            commands.entity(entity).try_despawn();
        }
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::SystemState;

    use super::*;

    #[test]
    fn position_updates_use_the_same_entity_before_and_after_commands_apply() {
        let mut world = World::new();
        let mut state = SystemState::<Commands>::new(&mut world);
        let mut entities = ClientEntities::default();
        // This ID would previously become the fabricated local entity 10747v0.
        let id = EntityId(747);
        let fabricated = Entity::from_raw_u32(10_747).unwrap();
        assert!(world.get_entity(fabricated).is_err());
        let entity;
        {
            let mut commands = state.get_mut(&mut world).unwrap();
            // Bevy accepts this ID for commands even though no spawn was queued.
            assert!(commands.get_entity(fabricated).is_ok());
            entity = entities.get_or_spawn(id, &mut commands).id();
            assert_ne!(entity, fabricated);
            for x in 0..3 {
                let mut player = entities.get_or_spawn(id, &mut commands);
                assert_eq!(player.id(), entity);
                player.insert(Transform::from_xyz(x as f32, 0.0, 0.0));
            }
        }
        state.apply(&mut world);
        assert_eq!(world.query::<&Transform>().iter(&world).count(), 1);
        assert_eq!(world.get::<Transform>(entity).unwrap().translation.x, 2.0);
        {
            let mut commands = state.get_mut(&mut world).unwrap();
            assert_eq!(entities.get_or_spawn(id, &mut commands).id(), entity);
            entities.remove(id, &mut commands);
            entities.remove(id, &mut commands);
            entities.remove(EntityId(999), &mut commands);
        }
        state.apply(&mut world);
        assert!(world.get_entity(entity).is_err());
    }

    #[test]
    fn delete_and_respawn_in_one_batch_allocates_a_new_entity() {
        let mut world = World::new();
        let mut state = SystemState::<Commands>::new(&mut world);
        let mut entities = ClientEntities::default();
        let (old, new);
        {
            let mut commands = state.get_mut(&mut world).unwrap();
            old = entities.get_or_spawn(EntityId(1), &mut commands).id();
            entities.remove(EntityId(1), &mut commands);
            new = entities.get_or_spawn(EntityId(1), &mut commands).id();
        }
        state.apply(&mut world);
        assert_ne!(old, new);
        assert!(world.get_entity(old).is_err());
        assert!(world.get::<DespawnOnWorldUnload>(new).is_some());
    }

    #[test]
    fn world_unload_clears_mapping_including_position_only_entities() {
        let mut world = World::new();
        let mut state = SystemState::<Commands>::new(&mut world);
        let mut entities = ClientEntities::default();
        let old = {
            let mut commands = state.get_mut(&mut world).unwrap();
            entities.get_or_spawn(EntityId(1), &mut commands).id()
        };
        state.apply(&mut world);
        world.despawn(old);
        entities.clear();
        let new = {
            let mut commands = state.get_mut(&mut world).unwrap();
            entities.get_or_spawn(EntityId(1), &mut commands).id()
        };
        state.apply(&mut world);
        assert_ne!(old, new);
        assert!(world.get::<Transform>(new).is_some());
    }
}
