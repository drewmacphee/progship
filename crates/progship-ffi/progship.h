#ifndef PROGSHIP_H
#define PROGSHIP_H

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * Opaque handle to the simulation engine
 */
typedef SimulationEngine *ProgShipHandle;

/**
 * Simulation statistics
 */
typedef struct ProgShipStats {
  /**
   * Current simulation time in hours
   */
  double sim_time_hours;
  /**
   * Number of crew members
   */
  uint32_t crew_count;
  /**
   * Number of passengers
   */
  uint32_t passenger_count;
  /**
   * Number of rooms
   */
  uint32_t room_count;
  /**
   * Number of active conversations
   */
  uint32_t conversation_count;
  /**
   * Number of pending maintenance tasks
   */
  uint32_t maintenance_count;
  /**
   * Current time scale
   */
  float time_scale;
} ProgShipStats;

/**
 * Person data returned to C
 */
typedef struct ProgShipPerson {
  /**
   * Index of this person (0 to person_count-1)
   */
  uint32_t index;
  /**
   * World X coordinate
   */
  float world_x;
  /**
   * World Y coordinate
   */
  float world_y;
  /**
   * Room ID the person is in
   */
  uint32_t room_id;
  /**
   * Deck level (0-indexed)
   */
  int32_t deck_level;
  /**
   * 1 if crew, 0 if passenger
   */
  uint8_t is_crew;
  /**
   * Hunger need (0.0 = satisfied, 1.0 = starving)
   */
  float hunger;
  /**
   * Fatigue need (0.0 = rested, 1.0 = exhausted)
   */
  float fatigue;
  /**
   * Social need (0.0 = satisfied, 1.0 = lonely)
   */
  float social;
} ProgShipPerson;

/**
 * Room data returned to C
 */
typedef struct ProgShipRoom {
  /**
   * Room ID (index)
   */
  uint32_t id;
  /**
   * World X position (center)
   */
  float world_x;
  /**
   * World Y position (center)
   */
  float world_y;
  /**
   * Room width in meters
   */
  float width;
  /**
   * Room depth in meters
   */
  float depth;
  /**
   * Deck level
   */
  int32_t deck_level;
  /**
   * Room type (see RoomType enum values)
   */
  uint8_t room_type;
} ProgShipRoom;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

/**
 * Create a new simulation engine
 *
 * Returns a handle that must be freed with `progship_destroy`
 */
ProgShipHandle progship_create(void);

/**
 * Destroy a simulation engine and free its memory
 */
void progship_destroy(ProgShipHandle handle);

/**
 * Generate a ship with the specified parameters
 *
 * # Parameters
 * - `num_decks`: Number of decks (1-10 recommended)
 * - `rooms_per_deck`: Rooms per deck (5-20 recommended)
 * - `passenger_capacity`: Number of passengers to generate
 * - `crew_size`: Number of crew members to generate
 */
void progship_generate(ProgShipHandle handle,
                       uint32_t num_decks,
                       uint32_t rooms_per_deck,
                       uint32_t passenger_capacity,
                       uint32_t crew_size);

/**
 * Update the simulation by delta_seconds (in real time)
 *
 * The actual simulation time advanced depends on the time scale.
 */
void progship_update(ProgShipHandle handle, float delta_seconds);

/**
 * Set the time scale (1.0 = real-time, 10.0 = 10x speed)
 */
void progship_set_time_scale(ProgShipHandle handle, float scale);

/**
 * Get current time scale
 */
float progship_get_time_scale(ProgShipHandle handle);

/**
 * Get simulation statistics
 */
bool progship_get_stats(ProgShipHandle handle, struct ProgShipStats *stats);

/**
 * Get the total number of people (crew + passengers)
 */
uint32_t progship_person_count(ProgShipHandle handle);

/**
 * Get person data by index
 *
 * Returns true if successful, false if index out of bounds
 */
bool progship_get_person(ProgShipHandle handle, uint32_t index, struct ProgShipPerson *person);

/**
 * Get the number of rooms
 */
uint32_t progship_room_count(ProgShipHandle handle);

/**
 * Get room data by index
 */
bool progship_get_room(ProgShipHandle handle, uint32_t index, struct ProgShipRoom *room);

/**
 * Get the number of decks
 */
uint32_t progship_deck_count(ProgShipHandle handle);

/**
 * Get ship dimensions
 */
bool progship_get_ship_dimensions(ProgShipHandle handle, float *length, float *width);

/**
 * Get the current simulation time as hours since start
 */
double progship_get_sim_time(ProgShipHandle handle);

/**
 * Get the current hour of day (0-23)
 */
uint32_t progship_get_hour_of_day(ProgShipHandle handle);

#ifdef __cplusplus
} // extern "C"
#endif // __cplusplus

#endif /* PROGSHIP_H */
