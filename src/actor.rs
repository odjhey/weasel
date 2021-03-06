//! Entities that can activate abilities.

use crate::ability::{AbilitiesAlteration, AbilitiesSeed, Ability, AbilityId, Activation};
use crate::battle::{Battle, BattleRules, BattleState};
use crate::character::Character;
use crate::entity::{Entities, EntityId};
use crate::entropy::Entropy;
use crate::error::{WeaselError, WeaselResult};
use crate::event::{Event, EventKind, EventProcessor, EventQueue, EventTrigger};
use crate::metric::WriteMetrics;
use crate::team::TeamId;
use crate::util::Id;
#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::fmt::{Debug, Formatter, Result};

/// A trait for objects which possess abilities and can act during a round.
pub trait Actor<R: BattleRules>: Character<R> {
    /// Returns an iterator over abilities.
    fn abilities<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Ability<R>> + 'a>;

    /// Returns the ability with the given id.
    fn ability(&self, id: &AbilityId<R>) -> Option<&Ability<R>>;

    /// Returns a mutable reference to the ability with the given id.
    fn ability_mut(&mut self, id: &AbilityId<R>) -> Option<&mut Ability<R>>;

    /// Adds a new ability. Replaces an existing ability with the same id.
    /// Returns the replaced ability, if present.
    fn add_ability(&mut self, ability: Ability<R>) -> Option<Ability<R>>;

    /// Removes an ability.
    /// Returns the removed ability, if present.
    fn remove_ability(&mut self, id: &AbilityId<R>) -> Option<Ability<R>>;

    /// Returns the id of the team to which this actor belongs.
    fn team_id(&self) -> &TeamId<R>;
}

/// Set of rules that handle how abilities are represented and how they can alter
/// the state of the world when activated.
pub trait ActorRules<R: BattleRules> {
    #[cfg(not(feature = "serialization"))]
    /// See [Ability](../ability/type.Ability.html).
    type Ability: Id + 'static;
    #[cfg(feature = "serialization")]
    /// See [Ability](../ability/type.Ability.html).
    type Ability: Id + 'static + Serialize + for<'a> Deserialize<'a>;

    #[cfg(not(feature = "serialization"))]
    /// See [AbilitiesSeed](../ability/type.AbilitiesSeed.html).
    type AbilitiesSeed: Clone + Debug;
    #[cfg(feature = "serialization")]
    /// See [AbilitiesSeed](../ability/type.AbilitiesSeed.html).
    type AbilitiesSeed: Clone + Debug + Serialize + for<'a> Deserialize<'a>;

    #[cfg(not(feature = "serialization"))]
    /// See [Activation](../ability/type.Activation.html).
    type Activation: Clone + Debug;
    #[cfg(feature = "serialization")]
    /// See [Activation](../ability/type.Activation.html).
    type Activation: Clone + Debug + Serialize + for<'a> Deserialize<'a>;

    #[cfg(not(feature = "serialization"))]
    /// See [AbilitiesAlteration](../ability/type.AbilitiesAlteration.html).
    type AbilitiesAlteration: Clone + Debug;
    #[cfg(feature = "serialization")]
    /// See [AbilitiesAlteration](../ability/type.AbilitiesAlteration.html).
    type AbilitiesAlteration: Clone + Debug + Serialize + for<'a> Deserialize<'a>;

    /// Generates all abilities of an actor.
    /// Abilities should have unique ids, otherwise only the last entry will be persisted.
    ///
    /// The provided implementation generates an empty set of abilities.
    fn generate_abilities(
        &self,
        _seed: &Option<Self::AbilitiesSeed>,
        _entropy: &mut Entropy<R>,
        _metrics: &mut WriteMetrics<R>,
    ) -> Box<dyn Iterator<Item = Self::Ability>> {
        Box::new(std::iter::empty())
    }

    /// Returns true if the actor can activate this ability with the given activation profile.
    /// The ability is guaranteed to be known by the actor.
    ///
    /// The provided implementation accepts any activation.
    fn activable(&self, _action: Action<R>) -> bool {
        true
    }

    /// Activate an ability.
    /// `ability` is guaranteed to be known by `actor`.\
    /// In order to change the state of the world, abilities should insert
    /// event prototypes in `event_queue`.
    ///
    /// The provided implementation does nothing.
    fn activate(
        &self,
        _state: &BattleState<R>,
        _action: Action<R>,
        _event_queue: &mut Option<EventQueue<R>>,
        _entropy: &mut Entropy<R>,
        _metrics: &mut WriteMetrics<R>,
    ) {
    }

    /// Alters one or more abilities starting from the given alteration object.
    ///
    /// The provided implementation does nothing.
    fn alter(
        &self,
        _actor: &mut dyn Actor<R>,
        _alteration: &Self::AbilitiesAlteration,
        _entropy: &mut Entropy<R>,
        _metrics: &mut WriteMetrics<R>,
    ) {
    }

    /// Invoked when a new round begins.
    ///
    /// The provided implementation does nothing.
    fn on_round_start(
        &self,
        _actor: &dyn Actor<R>,
        _event_queue: &mut Option<EventQueue<R>>,
        _entropy: &mut Entropy<R>,
        _metrics: &mut WriteMetrics<R>,
    ) {
    }

    /// Invoked when the current round ends.
    ///
    /// The provided implementation does nothing.
    fn on_round_end(
        &self,
        _actor: &dyn Actor<R>,
        _event_queue: &mut Option<EventQueue<R>>,
        _entropy: &mut Entropy<R>,
        _metrics: &mut WriteMetrics<R>,
    ) {
    }
}

/// An action is comprised by an actor who activates an ability with a given activation profile.
pub struct Action<'a, R: BattleRules> {
    /// The actor who is activating the ability.
    pub actor: &'a dyn Actor<R>,
    /// The ability.
    pub ability: &'a Ability<R>,
    /// The activation profile for the ability.
    pub activation: &'a Option<Activation<R>>,
}

impl<'a, R: BattleRules> Action<'a, R> {
    /// Creates a new action.
    pub fn new(
        actor: &'a dyn Actor<R>,
        ability: &'a Ability<R>,
        activation: &'a Option<Activation<R>>,
    ) -> Action<'a, R> {
        Action {
            actor,
            ability,
            activation,
        }
    }
}

/// An event to alter the abilities of an actor.
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct AlterAbilities<R: BattleRules> {
    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "EntityId<R>: Serialize",
            deserialize = "EntityId<R>: Deserialize<'de>"
        ))
    )]
    id: EntityId<R>,

    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "AbilitiesAlteration<R>: Serialize",
            deserialize = "AbilitiesAlteration<R>: Deserialize<'de>"
        ))
    )]
    alteration: AbilitiesAlteration<R>,
}

impl<R: BattleRules> AlterAbilities<R> {
    /// Returns a trigger for this event.
    pub fn trigger<'a, P: EventProcessor<R>>(
        processor: &'a mut P,
        id: EntityId<R>,
        alteration: AbilitiesAlteration<R>,
    ) -> AlterAbilitiesTrigger<'a, R, P> {
        AlterAbilitiesTrigger {
            processor,
            id,
            alteration,
        }
    }

    /// Returns the actor's entity id.
    pub fn id(&self) -> &EntityId<R> {
        &self.id
    }

    /// Returns the definition of the changes to the actor's abilities.
    pub fn alteration(&self) -> &AbilitiesAlteration<R> {
        &self.alteration
    }
}

impl<R: BattleRules> Debug for AlterAbilities<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "AlterAbilities {{ id: {:?}, alteration: {:?} }}",
            self.id, self.alteration
        )
    }
}

impl<R: BattleRules> Clone for AlterAbilities<R> {
    fn clone(&self) -> Self {
        AlterAbilities {
            id: self.id.clone(),
            alteration: self.alteration.clone(),
        }
    }
}

impl<R: BattleRules + 'static> Event<R> for AlterAbilities<R> {
    fn verify(&self, battle: &Battle<R>) -> WeaselResult<(), R> {
        verify_is_actor(battle.entities(), &self.id)
    }

    fn apply(&self, battle: &mut Battle<R>, _: &mut Option<EventQueue<R>>) {
        // Retrieve the actor.
        let actor = battle
            .state
            .entities
            .actor_mut(&self.id)
            .unwrap_or_else(|| panic!("constraint violated: actor {:?} not found", self.id));
        // Alter the actor.
        battle.rules.actor_rules().alter(
            actor,
            &self.alteration,
            &mut battle.entropy,
            &mut battle.metrics.write_handle(),
        );
    }

    fn kind(&self) -> EventKind {
        EventKind::AlterAbilities
    }

    fn box_clone(&self) -> Box<dyn Event<R>> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Trigger to build and fire an `AlterAbilities` event.
pub struct AlterAbilitiesTrigger<'a, R, P>
where
    R: BattleRules,
    P: EventProcessor<R>,
{
    processor: &'a mut P,
    id: EntityId<R>,
    alteration: AbilitiesAlteration<R>,
}

impl<'a, R, P> EventTrigger<'a, R, P> for AlterAbilitiesTrigger<'a, R, P>
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    fn processor(&'a mut self) -> &'a mut P {
        self.processor
    }

    /// Returns an `AlterAbilities` event.
    fn event(&self) -> Box<dyn Event<R>> {
        Box::new(AlterAbilities {
            id: self.id.clone(),
            alteration: self.alteration.clone(),
        })
    }
}

/// An event to regenerate the abilities of an actor.
///
/// A new set of abilities is created from a seed.\
/// - Abilities already known by the actor won't be modified.
/// - Abilities that the actor didn't have before will be added.
/// - Current actor's abilities that are not present in the new set will be removed
///   from the actor.
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct RegenerateAbilities<R: BattleRules> {
    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "EntityId<R>: Serialize",
            deserialize = "EntityId<R>: Deserialize<'de>"
        ))
    )]
    id: EntityId<R>,

    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "Option<AbilitiesSeed<R>>: Serialize",
            deserialize = "Option<AbilitiesSeed<R>>: Deserialize<'de>"
        ))
    )]
    seed: Option<AbilitiesSeed<R>>,
}

impl<R: BattleRules> RegenerateAbilities<R> {
    /// Returns a trigger for this event.
    pub fn trigger<P: EventProcessor<R>>(
        processor: &'_ mut P,
        id: EntityId<R>,
    ) -> RegenerateAbilitiesTrigger<'_, R, P> {
        RegenerateAbilitiesTrigger {
            processor,
            id,
            seed: None,
        }
    }

    /// Returns the actor's entity id.
    pub fn id(&self) -> &EntityId<R> {
        &self.id
    }

    /// Returns the seed to regenerate the actor's abilities.
    pub fn seed(&self) -> &Option<AbilitiesSeed<R>> {
        &self.seed
    }
}

impl<R: BattleRules> Debug for RegenerateAbilities<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "RegenerateAbilities {{ id: {:?}, seed: {:?} }}",
            self.id, self.seed
        )
    }
}

impl<R: BattleRules> Clone for RegenerateAbilities<R> {
    fn clone(&self) -> Self {
        RegenerateAbilities {
            id: self.id.clone(),
            seed: self.seed.clone(),
        }
    }
}

impl<R: BattleRules + 'static> Event<R> for RegenerateAbilities<R> {
    fn verify(&self, battle: &Battle<R>) -> WeaselResult<(), R> {
        verify_is_actor(battle.entities(), &self.id)
    }

    fn apply(&self, battle: &mut Battle<R>, _: &mut Option<EventQueue<R>>) {
        // Retrieve the actor.
        let actor = battle
            .state
            .entities
            .actor_mut(&self.id)
            .unwrap_or_else(|| panic!("constraint violated: actor {:?} not found", self.id));
        // Generate a new set of abilities.
        let abilities: Vec<_> = battle
            .rules
            .actor_rules()
            .generate_abilities(
                &self.seed,
                &mut battle.entropy,
                &mut battle.metrics.write_handle(),
            )
            .collect();
        let mut to_remove = Vec::new();
        // Remove all actor's abilities not present in the new set.
        for ability in actor.abilities() {
            if abilities.iter().find(|e| e.id() == ability.id()).is_none() {
                to_remove.push(ability.id().clone());
            }
        }
        for ability_id in to_remove {
            actor.remove_ability(&ability_id);
        }
        // Add all abilities present in the new set but not in the actor.
        for ability in abilities {
            if actor.ability(ability.id()).is_none() {
                actor.add_ability(ability);
            }
        }
    }

    fn kind(&self) -> EventKind {
        EventKind::RegenerateAbilities
    }

    fn box_clone(&self) -> Box<dyn Event<R>> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Trigger to build and fire a `RegenerateAbilities` event.
pub struct RegenerateAbilitiesTrigger<'a, R, P>
where
    R: BattleRules,
    P: EventProcessor<R>,
{
    processor: &'a mut P,
    id: EntityId<R>,
    seed: Option<AbilitiesSeed<R>>,
}

impl<'a, R, P> RegenerateAbilitiesTrigger<'a, R, P>
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    /// Adds a seed to drive the regeneration of this actor's abilities.
    pub fn seed(
        &'a mut self,
        seed: AbilitiesSeed<R>,
    ) -> &'a mut RegenerateAbilitiesTrigger<'a, R, P> {
        self.seed = Some(seed);
        self
    }
}

impl<'a, R, P> EventTrigger<'a, R, P> for RegenerateAbilitiesTrigger<'a, R, P>
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    fn processor(&'a mut self) -> &'a mut P {
        self.processor
    }

    /// Returns a `RegenerateAbilities` event.
    fn event(&self) -> Box<dyn Event<R>> {
        Box::new(RegenerateAbilities {
            id: self.id.clone(),
            seed: self.seed.clone(),
        })
    }
}

/// Checks if an entity exists and is an actor.
fn verify_is_actor<R>(entities: &Entities<R>, id: &EntityId<R>) -> WeaselResult<(), R>
where
    R: BattleRules,
{
    // Check if this entity claims to be an actor.
    if !id.is_actor() {
        return Err(WeaselError::NotAnActor(id.clone()));
    }
    // Check if the entity exists.
    entities
        .entity(id)
        .ok_or_else(|| WeaselError::EntityNotFound(id.clone()))?;
    Ok(())
}
