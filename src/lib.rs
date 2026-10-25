//! # `rust-zero-drift` — Reference Architectural Primitives
//!
//! Zero-dependency, `#![forbid(unsafe_code)]` reference implementations of the four core
//! **Zero-Drift Rust** compile-time guardrail patterns:
//!
//! 1. **Validated `#[repr(transparent)]` Newtypes ([`BoundedScalar`]):** Makes `NaN`, `Infinity`,
//!    and out-of-range domain values physically unconstructable at the type level.
//! 2. **Zero-Cost Typestate Workflows ([`Command`]):** Encodes protocol state machines into
//!    zero-sized `PhantomData` markers ([`Unchecked`] $\rightarrow$ [`Verified`] $\rightarrow$ [`Committed`])
//!    so out-of-order execution cannot compile.
//! 3. **Generational Arena Indices ([`GenerationalArena`], [`NodeKey`]):** Solves cyclic/bidirectional
//!    graph state (simulations, robotics transforms, UI trees) in flat contiguous memory without
//!    `Rc<RefCell<T>>` or `unsafe` pointers, while preventing ABA use-after-free bugs.
//! 4. **Sans-IO Deterministic Engine Kernel ([`SafetyGovernor`]):** Pure `Event -> Action` state
//!    transition logic decoupled from network, timer, or hardware I/O.

#![forbid(unsafe_code)]

use core::fmt;
use core::marker::PhantomData;

/// Exhaustive domain error contract for zero-drift state transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZeroDriftError {
    /// Floating-point input was `NaN` or infinite.
    NonFiniteScalar,
    /// Scalar magnitude exceeded the configured physical or risk ceiling.
    LimitExceeded {
        /// Maximum permitted magnitude in micro-units.
        max_micros: i64,
        /// Requested magnitude in micro-units.
        requested_micros: i64,
    },
    /// Attempted to submit a command while the governor circuit breaker is tripped.
    CircuitBreakerTripped {
        /// Consecutive fault count that tripped the breaker.
        fault_count: u32,
    },
    /// Attempted to access an arena slot using a stale or freed generational key.
    StaleArenaKey,
}

impl fmt::Display for ZeroDriftError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteScalar => f.write_str("scalar value must be finite (neither NaN nor infinite)"),
            Self::LimitExceeded {
                max_micros,
                requested_micros,
            } => write!(
                f,
                "requested magnitude ({requested_micros} micro-units) exceeds safety ceiling ({max_micros} micro-units)"
            ),
            Self::CircuitBreakerTripped { fault_count } => write!(
                f,
                "safety circuit breaker is tripped after {fault_count} consecutive violations"
            ),
            Self::StaleArenaKey => f.write_str("arena key generation mismatch (slot was freed or overwritten)"),
        }
    }
}

// ============================================================================
// Pattern 1: Validated #[repr(transparent)] Fixed-Point Newtype
// ============================================================================

/// A validated fixed-point scalar stored as exact integer micro-units (`10^-6`),
/// eliminating IEEE-754 floating-point drift while guaranteeing zero wrapper overhead.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BoundedScalar(i64);

impl BoundedScalar {
    /// Constructs a [`BoundedScalar`] from exact integer micro-units if within `[-max_abs_micros, +max_abs_micros]`.
    ///
    /// # Errors
    /// Returns [`ZeroDriftError::LimitExceeded`] if `micros.saturating_abs() > max_abs_micros.saturating_abs()`.
    #[inline]
    pub const fn try_from_micros(micros: i64, max_abs_micros: i64) -> Result<Self, ZeroDriftError> {
        let ceiling = max_abs_micros.saturating_abs();
        if micros.saturating_abs() <= ceiling {
            Ok(Self(micros))
        } else {
            Err(ZeroDriftError::LimitExceeded {
                max_micros: ceiling,
                requested_micros: micros,
            })
        }
    }

    /// Returns the underlying value in exact micro-units (`10^-6`).
    #[inline]
    #[must_use]
    pub const fn as_micros(self) -> i64 {
        self.0
    }
}

// ============================================================================
// Pattern 2: Zero-Cost Typestate Workflow (`Unchecked` -> `Verified` -> `Committed`)
// ============================================================================

/// Typestate marker: command payload has been ingested from an untrusted source and is not yet verified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unchecked;

/// Typestate marker: command payload has passed all domain and risk-ceiling invariants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Verified;

/// Typestate marker: command has been dispatched and committed with a monotonic sequence number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Committed;

/// A zero-drift command envelope whose legal lifecycle transitions are enforced at compile time.
///
/// Because [`PhantomData`] is a zero-sized type (ZST), `Command<State>` has the exact same
/// in-memory footprint across all states (`16 bytes` on 64-bit targets).
#[derive(Debug, PartialEq, Eq)]
pub struct Command<State = Unchecked> {
    sequence_id: u64,
    requested_micros: i64,
    _state: PhantomData<State>,
}

impl Command<Unchecked> {
    /// Creates a new [`Unchecked`] command envelope.
    ///
    /// Note that `.commit()` does **not** exist on `Command<Unchecked>`—attempting to execute
    /// an unverified command is a compile-time type error.
    #[inline]
    #[must_use]
    pub const fn new(sequence_id: u64, requested_micros: i64) -> Self {
        Self {
            sequence_id,
            requested_micros,
            _state: PhantomData,
        }
    }

    /// Consumes the [`Unchecked`] command by value and transitions to [`Command<Verified>`]
    /// if and only if the payload satisfies `max_abs_micros`.
    ///
    /// # Errors
    /// Returns [`ZeroDriftError::LimitExceeded`] if `requested_micros` violates the safety ceiling.
    #[inline]
    pub const fn verify(self, max_abs_micros: i64) -> Result<Command<Verified>, ZeroDriftError> {
        match BoundedScalar::try_from_micros(self.requested_micros, max_abs_micros) {
            Ok(valid) => Ok(Command {
                sequence_id: self.sequence_id,
                requested_micros: valid.as_micros(),
                _state: PhantomData,
            }),
            Err(err) => Err(err),
        }
    }
}

impl Command<Verified> {
    /// Returns the verified payload in micro-units.
    #[inline]
    #[must_use]
    pub const fn verified_micros(&self) -> i64 {
        self.requested_micros
    }

    /// Returns the command sequence identifier.
    #[inline]
    #[must_use]
    pub const fn sequence_id(&self) -> u64 {
        self.sequence_id
    }

    /// Consumes the [`Verified`] command and transitions to [`Command<Committed>`].
    #[inline]
    #[must_use]
    pub const fn commit(self) -> Command<Committed> {
        Command {
            sequence_id: self.sequence_id,
            requested_micros: self.requested_micros,
            _state: PhantomData,
        }
    }
}

impl Command<Committed> {
    /// Returns the committed `(sequence_id, verified_micros)` receipt tuple.
    #[inline]
    #[must_use]
    pub const fn receipt(&self) -> (u64, i64) {
        (self.sequence_id, self.requested_micros)
    }
}

// ============================================================================
// Pattern 3: Generational Arena (Cycle-Free Graph Storage Without `unsafe`)
// ============================================================================

/// A lightweight `Copy` handle into a [`GenerationalArena`] combining a 32-bit slot index
/// and a 32-bit generation counter (`8 bytes` total) to prevent ABA use-after-free bugs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeKey {
    index: u32,
    generation: u32,
}

#[derive(Debug, Clone)]
enum Slot<T> {
    Occupied { generation: u32, value: T },
    Vacant { generation: u32, next_free: Option<u32> },
}

/// Contiguous, cache-friendly generational arena for storing cyclic graphs, scene trees,
/// or state nodes without `Rc<RefCell<T>>` or raw pointers.
#[derive(Debug, Clone)]
pub struct GenerationalArena<T> {
    slots: Vec<Slot<T>>,
    free_head: Option<u32>,
    len: usize,
}

impl<T> Default for GenerationalArena<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> GenerationalArena<T> {
    /// Creates an empty [`GenerationalArena`].
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            slots: Vec::new(),
            free_head: None,
            len: 0,
        }
    }

    /// Returns the number of live elements currently stored in the arena.
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the arena contains no live elements.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Inserts `value` into the arena and returns its unique [`NodeKey`].
    ///
    /// # Errors
    /// Returns [`ZeroDriftError::LimitExceeded`] if the arena exceeds `u32::MAX` slots.
    pub fn insert(&mut self, value: T) -> Result<NodeKey, ZeroDriftError> {
        if let Some(idx) = self.free_head {
            let slot_idx = idx as usize;
            if let Some(&Slot::Vacant {
                generation,
                next_free,
            }) = self.slots.get(slot_idx)
            {
                self.free_head = next_free;
                self.slots[slot_idx] = Slot::Occupied { generation, value };
                self.len = self.len.saturating_add(1);
                return Ok(NodeKey {
                    index: idx,
                    generation,
                });
            }
        }

        let Ok(index) = u32::try_from(self.slots.len()) else {
            return Err(ZeroDriftError::LimitExceeded {
                max_micros: i64::from(u32::MAX),
                requested_micros: i64::MAX,
            });
        };

        let generation = 1;
        self.slots.push(Slot::Occupied { generation, value });
        self.len = self.len.saturating_add(1);
        Ok(NodeKey { index, generation })
    }

    /// Resolves a shared reference `&T` for `key` if its generation matches the live slot.
    #[inline]
    #[must_use]
    pub fn get(&self, key: NodeKey) -> Option<&T> {
        match self.slots.get(key.index as usize)? {
            Slot::Occupied { generation, value } if *generation == key.generation => Some(value),
            _ => None,
        }
    }

    /// Resolves an exclusive reference `&mut T` for `key` if its generation matches the live slot.
    #[inline]
    #[must_use]
    pub fn get_mut(&mut self, key: NodeKey) -> Option<&mut T> {
        match self.slots.get_mut(key.index as usize)? {
            Slot::Occupied { generation, value } if *generation == key.generation => Some(value),
            _ => None,
        }
    }

    /// Removes the element at `key` and bumps the slot's generation counter so stale keys
    /// can never alias a future allocation at the same index.
    ///
    /// # Errors
    /// Returns [`ZeroDriftError::StaleArenaKey`] if `key` was already removed or invalid.
    pub fn remove(&mut self, key: NodeKey) -> Result<T, ZeroDriftError> {
        let slot_idx = key.index as usize;
        let Some(slot) = self.slots.get_mut(slot_idx) else {
            return Err(ZeroDriftError::StaleArenaKey);
        };

        match slot {
            Slot::Occupied { generation, .. } if *generation == key.generation => {
                let next_gen = generation.wrapping_add(1).max(1);
                let old_slot = core::mem::replace(
                    slot,
                    Slot::Vacant {
                        generation: next_gen,
                        next_free: self.free_head,
                    },
                );
                self.free_head = Some(key.index);
                self.len = self.len.saturating_sub(1);
                if let Slot::Occupied { value, .. } = old_slot {
                    Ok(value)
                } else {
                    Err(ZeroDriftError::StaleArenaKey)
                }
            }
            _ => Err(ZeroDriftError::StaleArenaKey),
        }
    }
}

// ============================================================================
// Pattern 4: Sans-IO Deterministic Engine Kernel
// ============================================================================

/// Pure input event for the Sans-IO [`SafetyGovernor`] state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GovernorEvent {
    /// Submit a candidate command `(sequence_id, requested_micros)`.
    SubmitCommand {
        /// Monotonic sequence number.
        sequence_id: u64,
        /// Requested force or order size in micro-units.
        requested_micros: i64,
    },
    /// Explicitly reset the circuit breaker after operator verification.
    ResetBreaker,
}

/// Pure output action emitted by the Sans-IO [`SafetyGovernor`] state machine.
#[derive(Debug, PartialEq, Eq)]
pub enum GovernorAction {
    /// Command passed all invariants and transitioned to [`Committed`].
    Dispatch(Command<Committed>),
    /// Command was rejected with a structured [`ZeroDriftError`].
    Reject {
        /// Sequence ID of the rejected command.
        sequence_id: u64,
        /// Exact rejection reason.
        reason: ZeroDriftError,
    },
    /// Circuit breaker was reset to nominal operation.
    BreakerReset,
}

/// Deterministic Sans-IO safety governor that processes [`GovernorEvent`]s into [`GovernorAction`]s
/// with zero I/O, zero heap allocations, and bounded fault tolerance.
#[derive(Debug, Clone)]
pub struct SafetyGovernor {
    max_abs_micros: i64,
    max_consecutive_faults: u32,
    consecutive_faults: u32,
    committed_count: u64,
}

impl SafetyGovernor {
    /// Creates a new [`SafetyGovernor`] with the specified magnitude ceiling and fault threshold.
    #[inline]
    #[must_use]
    pub const fn new(max_abs_micros: i64, max_consecutive_faults: u32) -> Self {
        Self {
            max_abs_micros: max_abs_micros.saturating_abs(),
            max_consecutive_faults,
            consecutive_faults: 0,
            committed_count: 0,
        }
    }

    /// Evaluates a single [`GovernorEvent`] deterministically and returns the resulting [`GovernorAction`].
    #[inline]
    pub const fn on_event(&mut self, event: GovernorEvent) -> GovernorAction {
        match event {
            GovernorEvent::ResetBreaker => {
                self.consecutive_faults = 0;
                GovernorAction::BreakerReset
            }
            GovernorEvent::SubmitCommand {
                sequence_id,
                requested_micros,
            } => {
                if self.consecutive_faults >= self.max_consecutive_faults {
                    return GovernorAction::Reject {
                        sequence_id,
                        reason: ZeroDriftError::CircuitBreakerTripped {
                            fault_count: self.consecutive_faults,
                        },
                    };
                }

                let unchecked = Command::new(sequence_id, requested_micros);
                match unchecked.verify(self.max_abs_micros) {
                    Ok(verified) => {
                        self.consecutive_faults = 0;
                        self.committed_count = self.committed_count.saturating_add(1);
                        GovernorAction::Dispatch(verified.commit())
                    }
                    Err(reason) => {
                        self.consecutive_faults = self.consecutive_faults.saturating_add(1);
                        GovernorAction::Reject {
                            sequence_id,
                            reason,
                        }
                    }
                }
            }
        }
    }

    /// Returns the total number of commands successfully committed by this governor.
    #[inline]
    #[must_use]
    pub const fn committed_count(&self) -> u64 {
        self.committed_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::size_of;

    #[test]
    fn zero_cost_memory_layout_invariants() {
        // #[repr(transparent)] Newtype must have the exact byte size of i64 (8 bytes)
        assert_eq!(size_of::<BoundedScalar>(), size_of::<i64>());

        // Niche optimization: Option<&Command<Verified>> must equal a single pointer (8 bytes)
        assert_eq!(
            size_of::<Option<&Command<Verified>>>(),
            size_of::<&Command<Verified>>()
        );

        // Typestate PhantomData markers must add 0 bytes across all lifecycle states
        assert_eq!(size_of::<Command<Unchecked>>(), 16);
        assert_eq!(size_of::<Command<Verified>>(), 16);
        assert_eq!(size_of::<Command<Committed>>(), 16);
    }

    #[test]
    fn typestate_and_sans_io_governor_circuit_breaker() {
        let mut gov = SafetyGovernor::new(50_000_000, 2);

        // 1. Valid command transitions Unchecked -> Verified -> Committed
        let action_ok = gov.on_event(GovernorEvent::SubmitCommand {
            sequence_id: 1,
            requested_micros: 25_000_000,
        });
        assert_eq!(
            action_ok,
            GovernorAction::Dispatch(Command {
                sequence_id: 1,
                requested_micros: 25_000_000,
                _state: PhantomData,
            })
        );
        assert_eq!(gov.committed_count(), 1);

        // 2. Two consecutive out-of-bounds commands trip the circuit breaker
        let reject_1 = gov.on_event(GovernorEvent::SubmitCommand {
            sequence_id: 2,
            requested_micros: 75_000_000,
        });
        assert!(matches!(
            reject_1,
            GovernorAction::Reject {
                sequence_id: 2,
                reason: ZeroDriftError::LimitExceeded { .. }
            }
        ));

        let reject_2 = gov.on_event(GovernorEvent::SubmitCommand {
            sequence_id: 3,
            requested_micros: -90_000_000,
        });
        assert!(matches!(
            reject_2,
            GovernorAction::Reject {
                sequence_id: 3,
                reason: ZeroDriftError::LimitExceeded { .. }
            }
        ));

        // 3. Even a valid command is blocked while the circuit breaker is tripped
        let tripped = gov.on_event(GovernorEvent::SubmitCommand {
            sequence_id: 4,
            requested_micros: 10_000_000,
        });
        assert_eq!(
            tripped,
            GovernorAction::Reject {
                sequence_id: 4,
                reason: ZeroDriftError::CircuitBreakerTripped { fault_count: 2 },
            }
        );

        // 4. Explicit reset restores nominal operation
        assert_eq!(gov.on_event(GovernorEvent::ResetBreaker), GovernorAction::BreakerReset);
        let recovered = gov.on_event(GovernorEvent::SubmitCommand {
            sequence_id: 5,
            requested_micros: 10_000_000,
        });
        assert!(matches!(recovered, GovernorAction::Dispatch(_)));
        assert_eq!(gov.committed_count(), 2);
    }

    #[test]
    fn generational_arena_prevents_aba_stale_key_aliasing() {
        let mut arena = GenerationalArena::new();
        let key_a = arena.insert("Joint_A").unwrap_or_else(|_| unreachable!());
        assert_eq!(arena.get(key_a), Some(&"Joint_A"));

        // Remove Joint_A and insert Joint_B into the reused slot
        let removed = arena.remove(key_a);
        assert_eq!(removed, Ok("Joint_A"));

        let key_b = arena.insert("Joint_B").unwrap_or_else(|_| unreachable!());
        assert_eq!(key_a.index, key_b.index);
        assert_ne!(key_a.generation, key_b.generation);

        // Stale key_a must NEVER resolve to Joint_B!
        assert_eq!(arena.get(key_a), None);
        assert_eq!(arena.get(key_b), Some(&"Joint_B"));
    }
}
