//! Teams of entities.

use crate::battle::{Battle, BattleRules, BattleState};
use crate::creature::{Creature, CreatureId};
use crate::error::{WeaselError, WeaselResult};
use crate::event::{Event, EventKind, EventProcessor, EventQueue, EventTrigger};
use crate::metric::system::*;
use crate::metric::ReadMetrics;
use crate::util::Id;
#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Formatter, Result};
use std::hash::{Hash, Hasher};
use std::{any::Any, iter};

/// A team is an alliance of entities.
///
/// A team represents the unit of control of a player. Teams must achieve their objectives in
/// order to win the battle.
pub struct Team<R: BattleRules> {
    /// The id of this team.
    id: TeamId<R>,
    /// Ids of all creatures which are currently part of this team.
    creatures: Vec<CreatureId<R>>,
    /// `Conclusion`, if any, reached by this team.
    conclusion: Option<Conclusion>,
    /// Team objectives.
    objectives: Objectives<R>,
}

impl<R: BattleRules> Team<R> {
    /// Returns an iterator over creatures.
    pub fn creatures(&self) -> impl Iterator<Item = &CreatureId<R>> {
        Box::new(self.creatures.iter())
    }

    pub(crate) fn creatures_mut(&mut self) -> &mut Vec<CreatureId<R>> {
        &mut self.creatures
    }

    /// Returns the conclusion reached by this team, if any.
    pub fn conclusion(&self) -> Option<Conclusion> {
        self.conclusion
    }

    /// Returns the team's objectives.
    pub fn objectives(&self) -> &Objectives<R> {
        &self.objectives
    }

    /// Removes a creature id from this team.
    ///
    /// # Panics
    ///
    /// Panics if the given creature id is not part of the team.
    ///
    pub(crate) fn remove_creature(&mut self, creature: &CreatureId<R>) {
        let index = self.creatures.iter().position(|x| x == creature).expect(
            "constraint violated: creature is in a team, \
             but such team doesn't contain the creature",
        );
        self.creatures.remove(index);
    }
}

impl<R: BattleRules> Id for Team<R> {
    type Id = TeamId<R>;

    fn id(&self) -> &TeamId<R> {
        &self.id
    }
}

/// Collection of rules to manage teams of creatures.
pub trait TeamRules<R: BattleRules> {
    #[cfg(not(feature = "serialization"))]
    /// See [TeamId](type.TeamId.html).
    type Id: Hash + Eq + PartialOrd + Clone + Debug;
    #[cfg(feature = "serialization")]
    /// See [TeamId](type.TeamId.html).
    type Id: Hash + Eq + PartialOrd + Clone + Debug + Serialize + for<'a> Deserialize<'a>;

    #[cfg(not(feature = "serialization"))]
    /// See [ObjectivesSeed](type.ObjectivesSeed.html).
    type ObjectivesSeed: Clone + Debug;
    #[cfg(feature = "serialization")]
    /// See [ObjectivesSeed](type.ObjectivesSeed.html).
    type ObjectivesSeed: Clone + Debug + Serialize + for<'a> Deserialize<'a>;

    /// See [Objectives](type.Objectives.html).
    type Objectives: Default;

    /// Checks if the addition of a new entity in the given team is allowed.
    ///
    /// The provided implementation accepts any new entity.
    fn allow_new_entity(
        &self,
        _state: &BattleState<R>,
        _team: &Team<R>,
        _type: EntityAddition<R>,
    ) -> bool {
        true
    }

    /// Generate the objectives for a team.
    ///
    /// The provided implementation returns `Objectives::default()`.\
    /// If you set team `Conclusion` manually, you may avoid implementing this method.
    fn generate_objectives(&self, _seed: &Option<Self::ObjectivesSeed>) -> Self::Objectives {
        Self::Objectives::default()
    }

    /// Checks if the team has completed its objectives.
    /// This check is called after every event.
    ///
    /// The provided implementation does not return any conclusion.\
    /// If you set team `Conclusion` manually, you may avoid implementing this method.
    ///
    /// Returns the `Conclusion` for this team, or none if it did not reach any.
    fn check_objectives_on_event(
        &self,
        _state: &BattleState<R>,
        _team: &Team<R>,
        _metrics: &ReadMetrics<R>,
    ) -> Option<Conclusion> {
        None
    }

    /// Checks if the team has completed its objectives.
    /// This check is called every time a round ends.
    ///
    /// The provided implementation does not return any conclusion.\
    /// If you set team `Conclusion` manually, you may avoid implementing this method.
    ///
    /// Returns the `Conclusion` for this team, or none if it did not reach any.
    fn check_objectives_on_round(
        &self,
        _state: &BattleState<R>,
        _team: &Team<R>,
        _metrics: &ReadMetrics<R>,
    ) -> Option<Conclusion> {
        None
    }
}

/// Type to drive the generation of the objectives for a given team.
///
/// For instance, a seed might contain the identifiers of all enemies who must be defeated.
pub type ObjectivesSeed<R> = <<R as BattleRules>::TR as TeamRules<R>>::ObjectivesSeed;

/// Type to store all information about the objectives of a team.
///
/// The objectives can be checked during the battle to know whether or not a team is victorious.
pub type Objectives<R> = <<R as BattleRules>::TR as TeamRules<R>>::Objectives;

/// Describes the different scenarios in which an entity might be added to a team.
pub enum EntityAddition<'a, R: BattleRules> {
    /// Spawn a new creature.
    CreatureSpawn,
    /// Take a creature from another team.
    CreatureConversion(&'a Creature<R>),
}

/// Type to uniquely identify teams.
pub type TeamId<R> = <<R as BattleRules>::TR as TeamRules<R>>::Id;

/// Event to create a new team.
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct CreateTeam<R: BattleRules> {
    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "TeamId<R>: Serialize",
            deserialize = "TeamId<R>: Deserialize<'de>"
        ))
    )]
    id: TeamId<R>,

    /// Optional vector containing pairs of teams and relations.
    /// Set `relations` to explicitly set the relation betwen the newly created team
    /// and a list of existing teams.
    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "Option<Vec<(TeamId<R>, Relation)>>: Serialize",
            deserialize = "Option<Vec<(TeamId<R>, Relation)>>: Deserialize<'de>"
        ))
    )]
    relations: Option<Vec<(TeamId<R>, Relation)>>,

    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "Option<ObjectivesSeed<R>>: Serialize",
            deserialize = "Option<ObjectivesSeed<R>>: Deserialize<'de>"
        ))
    )]
    objectives_seed: Option<ObjectivesSeed<R>>,
}

impl<R: BattleRules> Debug for CreateTeam<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "CreateTeam {{ id: {:?}, relations: {:?}, objectives_seed: {:?} }}",
            self.id, self.relations, self.objectives_seed
        )
    }
}

impl<R: BattleRules> Clone for CreateTeam<R> {
    fn clone(&self) -> Self {
        CreateTeam {
            id: self.id.clone(),
            relations: self.relations.clone(),
            objectives_seed: self.objectives_seed.clone(),
        }
    }
}

impl<R: BattleRules> CreateTeam<R> {
    /// Returns a trigger for this event.
    pub fn trigger<'a, P: EventProcessor<R>>(
        processor: &'a mut P,
        id: TeamId<R>,
    ) -> CreateTeamTrigger<'a, R, P> {
        CreateTeamTrigger {
            processor,
            id,
            relations: None,
            objectives_seed: None,
        }
    }

    /// Returns the team id.
    pub fn id(&self) -> &TeamId<R> {
        &self.id
    }

    /// Returns the optional relations for the new team.
    pub fn relations(&self) -> &Option<Vec<(TeamId<R>, Relation)>> {
        &self.relations
    }

    /// Returns the seed to generate the team's objectives.
    pub fn objectives_seed(&self) -> &Option<ObjectivesSeed<R>> {
        &self.objectives_seed
    }
}

impl<R: BattleRules + 'static> Event<R> for CreateTeam<R> {
    fn verify(&self, battle: &Battle<R>) -> WeaselResult<(), R> {
        // New team must not already exist.
        if battle.entities().team(&self.id).is_some() {
            return Err(WeaselError::DuplicatedTeam(self.id.clone()));
        }
        if let Some(relations) = &self.relations {
            for (team_id, relation) in relations {
                // Prevent self relation assignment.
                if *team_id == self.id {
                    return Err(WeaselError::SelfRelation);
                }
                // Prevent explicit kinship.
                if *relation == Relation::Kin {
                    return Err(WeaselError::KinshipRelation);
                }
                // Teams in the relations list must exist.
                if battle.entities().team(&team_id).is_none() {
                    return Err(WeaselError::TeamNotFound(team_id.clone()));
                }
            }
        }
        Ok(())
    }

    fn apply(&self, battle: &mut Battle<R>, _: &mut Option<EventQueue<R>>) {
        // Insert the new team.
        battle.state.entities.add_team(Team {
            id: self.id.clone(),
            creatures: Vec::new(),
            conclusion: None,
            objectives: battle
                .rules
                .team_rules()
                .generate_objectives(&self.objectives_seed),
        });
        // Unpack explicit relations into a vector.
        let mut relations = if let Some(relations) = &self.relations {
            relations
                .iter()
                .map(|e| (RelationshipPair::new(self.id.clone(), e.0.clone()), e.1))
                .collect()
        } else {
            Vec::new()
        };
        // Set to `Relation::Enemy` all relations to other teams not explicitly set.
        for team_id in battle.entities().teams().map(|e| e.id()).filter(|e| {
            **e != self.id
                && self
                    .relations
                    .as_ref()
                    .unwrap_or(&Vec::new())
                    .iter()
                    .find(|(id, _)| *id == **e)
                    .is_none()
        }) {
            relations.push((
                RelationshipPair::new(self.id.clone(), team_id.clone()),
                Relation::Enemy,
            ));
        }
        // Insert the new relations.
        battle.state.entities.update_relations(relations);
        // Update metrics.
        battle
            .metrics
            .write_handle()
            .add_system_u64(TEAMS_CREATED, 1)
            .unwrap_or_else(|err| panic!("constraint violated: {:?}", err));
    }

    fn kind(&self) -> EventKind {
        EventKind::CreateTeam
    }

    fn box_clone(&self) -> Box<dyn Event<R>> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Trigger to build and fire a `CreateTeam` event.
pub struct CreateTeamTrigger<'a, R, P>
where
    R: BattleRules,
    P: EventProcessor<R>,
{
    processor: &'a mut P,
    id: TeamId<R>,
    relations: Option<Vec<(TeamId<R>, Relation)>>,
    objectives_seed: Option<ObjectivesSeed<R>>,
}

impl<'a, R, P> CreateTeamTrigger<'a, R, P>
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    /// Adds a list of relationships between this team and other existing teams.
    pub fn relations(
        &'a mut self,
        relations: &[(TeamId<R>, Relation)],
    ) -> &'a mut CreateTeamTrigger<'a, R, P> {
        self.relations = Some(relations.into());
        self
    }

    /// Adds a seed to drive the generation of this team objectives.
    pub fn objectives_seed(
        &'a mut self,
        seed: ObjectivesSeed<R>,
    ) -> &'a mut CreateTeamTrigger<'a, R, P> {
        self.objectives_seed = Some(seed);
        self
    }
}

impl<'a, R, P> EventTrigger<'a, R, P> for CreateTeamTrigger<'a, R, P>
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    fn processor(&'a mut self) -> &'a mut P {
        self.processor
    }

    /// Returns a `CreateTeam` event.
    fn event(&self) -> Box<dyn Event<R>> {
        Box::new(CreateTeam {
            id: self.id.clone(),
            relations: self.relations.clone(),
            objectives_seed: self.objectives_seed.clone(),
        })
    }
}

/// All possible kinds of relation between teams and thus entities.
#[derive(Debug, Copy, Clone, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub enum Relation {
    /// Represents an alliance.
    Ally,
    /// Represents enmity.
    Enemy,
    /// Reserved for entities in the same team.
    Kin,
}

/// A pair of two teams that are part of a relationship.
#[derive(Clone)]
pub(crate) struct RelationshipPair<R: BattleRules> {
    pub(crate) first: TeamId<R>,
    pub(crate) second: TeamId<R>,
}

impl<R: BattleRules> Debug for RelationshipPair<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "RelationshipPair {{ first: {:?}, second: {:?} }}",
            self.first, self.second
        )
    }
}

impl<R: BattleRules> RelationshipPair<R> {
    pub(crate) fn new(first: TeamId<R>, second: TeamId<R>) -> RelationshipPair<R> {
        RelationshipPair { first, second }
    }

    pub(crate) fn values(&self) -> impl Iterator<Item = TeamId<R>> {
        let first = iter::once(self.first.clone());
        let second = iter::once(self.second.clone());
        first.chain(second)
    }
}

impl<R: BattleRules> PartialEq for RelationshipPair<R> {
    fn eq(&self, other: &Self) -> bool {
        (self.first == other.first && self.second == other.second)
            || (self.first == other.second && self.second == other.first)
    }
}

impl<R: BattleRules> Eq for RelationshipPair<R> {}

impl<R: BattleRules> Hash for RelationshipPair<R> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if self.first > self.second {
            self.first.hash(state);
            self.second.hash(state);
        } else {
            self.second.hash(state);
            self.first.hash(state);
        }
    }
}

/// Event to set diplomatic relations between teams.
/// Relations are symmetric.
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct SetRelations<R: BattleRules> {
    /// Vector containing tuples of two teams and a relation.
    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "Vec<(TeamId<R>, TeamId<R>, Relation)>: Serialize",
            deserialize = "Vec<(TeamId<R>, TeamId<R>, Relation)>: Deserialize<'de>"
        ))
    )]
    relations: Vec<(TeamId<R>, TeamId<R>, Relation)>,
}

impl<R: BattleRules> Debug for SetRelations<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "SetRelations {{ relations: {:?} }}", self.relations)
    }
}

impl<R: BattleRules> Clone for SetRelations<R> {
    fn clone(&self) -> Self {
        SetRelations {
            relations: self.relations.clone(),
        }
    }
}

impl<R: BattleRules> SetRelations<R> {
    /// Returns a trigger for this event.
    pub fn trigger<'a, P: EventProcessor<R>>(
        processor: &'a mut P,
        relations: &[(TeamId<R>, TeamId<R>, Relation)],
    ) -> SetRelationsTrigger<'a, R, P> {
        SetRelationsTrigger {
            processor,
            relations: relations.into(),
        }
    }

    /// Returns all relation changes.
    pub fn relations(&self) -> &Vec<(TeamId<R>, TeamId<R>, Relation)> {
        &self.relations
    }
}

impl<R: BattleRules + 'static> Event<R> for SetRelations<R> {
    fn verify(&self, battle: &Battle<R>) -> WeaselResult<(), R> {
        for (first, second, relation) in &self.relations {
            // Prevent self relation assignment.
            if *first == *second {
                return Err(WeaselError::SelfRelation);
            }
            // Prevent explicit kinship.
            if *relation == Relation::Kin {
                return Err(WeaselError::KinshipRelation);
            }
            // Teams in the relations list must exist.
            if battle.entities().team(first).is_none() {
                return Err(WeaselError::TeamNotFound(first.clone()));
            }
            if battle.entities().team(second).is_none() {
                return Err(WeaselError::TeamNotFound(second.clone()));
            }
        }
        Ok(())
    }

    fn apply(&self, battle: &mut Battle<R>, _: &mut Option<EventQueue<R>>) {
        // Insert the new relations.
        let vec = self
            .relations
            .iter()
            .map(|e| (RelationshipPair::new(e.0.clone(), e.1.clone()), e.2))
            .collect();
        battle.state.entities.update_relations(vec);
    }

    fn kind(&self) -> EventKind {
        EventKind::SetRelations
    }

    fn box_clone(&self) -> Box<dyn Event<R>> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Trigger to build and fire a `SetRelations` event.
pub struct SetRelationsTrigger<'a, R, P>
where
    R: BattleRules,
    P: EventProcessor<R>,
{
    processor: &'a mut P,
    relations: Vec<(TeamId<R>, TeamId<R>, Relation)>,
}

impl<'a, R, P> EventTrigger<'a, R, P> for SetRelationsTrigger<'a, R, P>
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    fn processor(&'a mut self) -> &'a mut P {
        self.processor
    }

    /// Returns a `SetRelations` event.
    fn event(&self) -> Box<dyn Event<R>> {
        Box::new(SetRelations {
            relations: self.relations.clone(),
        })
    }
}

/// All possible conclusions for a team's objectives.
/// In other words, this tells if the team reached its objectives or failed.
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub enum Conclusion {
    /// Team achieved its objectives.
    Victory,
    /// Team failed to achieve its objectives.
    Defeat,
}

/// Event to set the `Conclusion` of a team.
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct ConcludeObjectives<R: BattleRules> {
    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "TeamId<R>: Serialize",
            deserialize = "TeamId<R>: Deserialize<'de>"
        ))
    )]
    id: TeamId<R>,

    conclusion: Conclusion,
}

impl<R: BattleRules> Debug for ConcludeObjectives<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "ConcludeObjectives {{ id: {:?}, conclusion: {:?} }}",
            self.id, self.conclusion
        )
    }
}

impl<R: BattleRules> Clone for ConcludeObjectives<R> {
    fn clone(&self) -> Self {
        ConcludeObjectives {
            id: self.id.clone(),
            conclusion: self.conclusion,
        }
    }
}

impl<R: BattleRules> ConcludeObjectives<R> {
    /// Returns a trigger for this event.
    pub fn trigger<'a, P: EventProcessor<R>>(
        processor: &'a mut P,
        id: TeamId<R>,
        conclusion: Conclusion,
    ) -> ConcludeMissionTrigger<'a, R, P> {
        ConcludeMissionTrigger {
            processor,
            id,
            conclusion,
        }
    }
}

impl<R: BattleRules + 'static> Event<R> for ConcludeObjectives<R> {
    fn verify(&self, battle: &Battle<R>) -> WeaselResult<(), R> {
        // Team must exist.
        if battle.entities().team(&self.id).is_none() {
            return Err(WeaselError::TeamNotFound(self.id.clone()));
        }
        Ok(())
    }

    fn apply(&self, battle: &mut Battle<R>, _: &mut Option<EventQueue<R>>) {
        // Change the team's conclusion.
        let team = battle
            .state
            .entities
            .team_mut(&self.id)
            .unwrap_or_else(|| panic!("constraint violated: team {:?} not found", self.id));
        team.conclusion = Some(self.conclusion);
    }

    fn kind(&self) -> EventKind {
        EventKind::ConcludeObjectives
    }

    fn box_clone(&self) -> Box<dyn Event<R>> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Trigger to build and fire a `ConcludeObjectives` event.
pub struct ConcludeMissionTrigger<'a, R, P>
where
    R: BattleRules,
    P: EventProcessor<R>,
{
    processor: &'a mut P,
    id: TeamId<R>,
    conclusion: Conclusion,
}

impl<'a, R, P> EventTrigger<'a, R, P> for ConcludeMissionTrigger<'a, R, P>
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    fn processor(&'a mut self) -> &'a mut P {
        self.processor
    }

    /// Returns a `ConcludeObjectives` event.
    fn event(&self) -> Box<dyn Event<R>> {
        Box::new(ConcludeObjectives {
            id: self.id.clone(),
            conclusion: self.conclusion,
        })
    }
}

/// Event to reset a team's objectives.
/// Team's `Conclusion` is resetted as well since the objectives changed.
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct ResetObjectives<R: BattleRules> {
    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "TeamId<R>: Serialize",
            deserialize = "TeamId<R>: Deserialize<'de>"
        ))
    )]
    id: TeamId<R>,

    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "Option<ObjectivesSeed<R>>: Serialize",
            deserialize = "Option<ObjectivesSeed<R>>: Deserialize<'de>"
        ))
    )]
    seed: Option<ObjectivesSeed<R>>,
}

impl<R: BattleRules> ResetObjectives<R> {
    /// Returns a trigger for this event.
    pub fn trigger<P: EventProcessor<R>>(
        processor: &mut P,
        id: TeamId<R>,
    ) -> ResetObjectivesTrigger<R, P> {
        ResetObjectivesTrigger {
            processor,
            id,
            seed: None,
        }
    }

    /// Returns the team id.
    pub fn id(&self) -> &TeamId<R> {
        &self.id
    }

    /// Returns the new seed.
    pub fn seed(&self) -> &Option<ObjectivesSeed<R>> {
        &self.seed
    }
}

impl<R: BattleRules> Debug for ResetObjectives<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "ResetObjectives {{ id: {:?}, seed: {:?} }}",
            self.id, self.seed
        )
    }
}

impl<R: BattleRules> Clone for ResetObjectives<R> {
    fn clone(&self) -> Self {
        ResetObjectives {
            id: self.id.clone(),
            seed: self.seed.clone(),
        }
    }
}

impl<R: BattleRules + 'static> Event<R> for ResetObjectives<R> {
    fn verify(&self, battle: &Battle<R>) -> WeaselResult<(), R> {
        // Team must exist.
        if battle.entities().team(&self.id).is_none() {
            return Err(WeaselError::TeamNotFound(self.id.clone()));
        }
        Ok(())
    }

    fn apply(&self, battle: &mut Battle<R>, _: &mut Option<EventQueue<R>>) {
        // Regenerate the team's objectives.
        let team = battle
            .state
            .entities
            .team_mut(&self.id)
            .unwrap_or_else(|| panic!("constraint violated: team {:?} not found", self.id));
        team.objectives = battle.rules.team_rules().generate_objectives(&self.seed);
        // Reset the team's conclusion.
        team.conclusion = None;
    }

    fn kind(&self) -> EventKind {
        EventKind::ResetObjectives
    }

    fn box_clone(&self) -> Box<dyn Event<R>> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Trigger to build and fire a `ResetObjectives` event.
pub struct ResetObjectivesTrigger<'a, R, P>
where
    R: BattleRules,
    P: EventProcessor<R>,
{
    processor: &'a mut P,
    id: TeamId<R>,
    seed: Option<ObjectivesSeed<R>>,
}

impl<'a, R, P> ResetObjectivesTrigger<'a, R, P>
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    /// Adds a seed to drive the generation of the new objectives.
    pub fn seed(&'a mut self, seed: ObjectivesSeed<R>) -> &'a mut ResetObjectivesTrigger<'a, R, P> {
        self.seed = Some(seed);
        self
    }
}

impl<'a, R, P> EventTrigger<'a, R, P> for ResetObjectivesTrigger<'a, R, P>
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    fn processor(&'a mut self) -> &'a mut P {
        self.processor
    }

    /// Returns a `ResetObjectives` event.
    fn event(&self) -> Box<dyn Event<R>> {
        Box::new(ResetObjectives {
            id: self.id.clone(),
            seed: self.seed.clone(),
        })
    }
}

/// Event to remove a team from a battle.
/// Teams can be removed only if they are empty.
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct RemoveTeam<R: BattleRules> {
    #[cfg_attr(
        feature = "serialization",
        serde(bound(
            serialize = "TeamId<R>: Serialize",
            deserialize = "TeamId<R>: Deserialize<'de>"
        ))
    )]
    id: TeamId<R>,
}

impl<R: BattleRules> RemoveTeam<R> {
    /// Returns a trigger for this event.
    pub fn trigger<P: EventProcessor<R>>(
        processor: &mut P,
        id: TeamId<R>,
    ) -> RemoveTeamTrigger<R, P> {
        RemoveTeamTrigger { processor, id }
    }

    /// Returns the id of the team to be removed.
    pub fn id(&self) -> &TeamId<R> {
        &self.id
    }
}

impl<R: BattleRules> Debug for RemoveTeam<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "RemoveTeam {{ id: {:?} }}", self.id)
    }
}

impl<R: BattleRules> Clone for RemoveTeam<R> {
    fn clone(&self) -> Self {
        RemoveTeam {
            id: self.id.clone(),
        }
    }
}

impl<R: BattleRules + 'static> Event<R> for RemoveTeam<R> {
    fn verify(&self, battle: &Battle<R>) -> WeaselResult<(), R> {
        // Team must exist.
        if let Some(team) = battle.entities().team(&self.id) {
            // Team must not have any creature.
            if team.creatures().peekable().peek().is_some() {
                return Err(WeaselError::TeamNotEmpty(self.id.clone()));
            }
            Ok(())
        } else {
            Err(WeaselError::TeamNotFound(self.id.clone()))
        }
    }

    fn apply(&self, battle: &mut Battle<R>, _: &mut Option<EventQueue<R>>) {
        // Remove the team.
        battle
            .state
            .entities
            .remove_team(&self.id)
            .unwrap_or_else(|err| panic!("constraint violated: {:?}", err));
        // Remove rights of players towards this team.
        battle.rights_mut().remove_team(&self.id);
    }

    fn kind(&self) -> EventKind {
        EventKind::RemoveTeam
    }

    fn box_clone(&self) -> Box<dyn Event<R>> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Trigger to build and fire a `RemoveTeam` event.
pub struct RemoveTeamTrigger<'a, R, P>
where
    R: BattleRules,
    P: EventProcessor<R>,
{
    processor: &'a mut P,
    id: TeamId<R>,
}

impl<'a, R, P> EventTrigger<'a, R, P> for RemoveTeamTrigger<'a, R, P>
where
    R: BattleRules + 'static,
    P: EventProcessor<R>,
{
    fn processor(&'a mut self) -> &'a mut P {
        self.processor
    }

    /// Returns a `RemoveTeam` event.
    fn event(&self) -> Box<dyn Event<R>> {
        Box::new(RemoveTeam {
            id: self.id.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{battle_rules, rules::empty::*};
    use std::collections::hash_map::DefaultHasher;

    fn get_hash<T: Hash>(item: &T) -> u64 {
        let mut hasher = DefaultHasher::new();
        item.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn relationship_hash_eq() {
        battle_rules! {}
        let r11 = RelationshipPair::<CustomRules>::new(1, 1);
        let r12 = RelationshipPair::<CustomRules>::new(1, 2);
        let r21 = RelationshipPair::<CustomRules>::new(2, 1);
        assert_eq!(r11, r11);
        assert_eq!(r12, r21);
        assert_ne!(r11, r12);
        assert_eq!(get_hash(&r11), get_hash(&r11));
        assert_eq!(get_hash(&r12), get_hash(&r21));
        assert_ne!(get_hash(&r11), get_hash(&r12));
    }
}
