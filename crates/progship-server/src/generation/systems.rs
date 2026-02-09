//! Ship systems and atmosphere generation.
//!
//! Creates ShipSystem/Subsystem/SystemComponent hierarchy with infrastructure
//! connectivity (InfraEdge) and per-deck atmosphere initialization.

use crate::tables::*;
use spacetimedb::{ReducerContext, Table};

pub(super) fn generate_ship_systems(ctx: &ReducerContext) {
    let insert_system = |name: &str, sys_type: u8, priority: u8| -> u64 {
        ctx.db
            .ship_system()
            .insert(ShipSystem {
                id: 0,
                name: name.to_string(),
                system_type: sys_type,
                overall_health: 1.0,
                overall_status: system_statuses::NOMINAL,
                priority,
            })
            .id
    };

    // Find node_id by room_type from the GraphNode entries
    let find_node = |func: u8| -> u64 {
        ctx.db
            .graph_node()
            .iter()
            .find(|n| n.function == func)
            .map(|n| n.id)
            .unwrap_or(0)
    };

    let reactor_node = find_node(room_types::REACTOR);
    let engineering_node = find_node(room_types::ENGINEERING);
    let power_dist_node = find_node(room_types::POWER_DISTRIBUTION);
    let ls_node = find_node(room_types::LIFE_SUPPORT);
    let cooling_node = find_node(room_types::COOLING_PLANT);
    let hvac_node = find_node(room_types::HVAC_CONTROL);
    let water_node = find_node(room_types::WATER_RECYCLING);
    let waste_node = find_node(room_types::WASTE_PROCESSING);
    let hydro_node = find_node(room_types::HYDROPONICS);
    let galley_node = find_node(room_types::GALLEY);
    let bridge_node = find_node(room_types::BRIDGE);
    let comms_node = find_node(room_types::COMMS_ROOM);
    let medical_node = find_node(room_types::HOSPITAL_WARD);

    let insert_subsystem = |system_id: u64,
                            name: &str,
                            sub_type: u8,
                            node_id: u64,
                            power_draw: f32,
                            crew_req: u8|
     -> u64 {
        ctx.db
            .subsystem()
            .insert(Subsystem {
                id: 0,
                system_id,
                name: name.to_string(),
                subsystem_type: sub_type,
                health: 1.0,
                status: system_statuses::NOMINAL,
                node_id,
                power_draw,
                crew_required: crew_req,
            })
            .id
    };

    let insert_component =
        |subsystem_id: u64, name: &str, comp_type: u8, px: f32, py: f32, maint_hours: f32| {
            ctx.db.system_component().insert(SystemComponent {
                id: 0,
                subsystem_id,
                name: name.to_string(),
                component_type: comp_type,
                health: 1.0,
                status: system_statuses::NOMINAL,
                position_x: px,
                position_y: py,
                maintenance_interval_hours: maint_hours,
                last_maintenance: 0.0,
            });
        };

    // Find the first service corridor for infra edge routing
    let svc_corridor_id = ctx
        .db
        .corridor()
        .iter()
        .find(|c| c.corridor_type == corridor_types::SERVICE)
        .map(|c| c.id)
        .unwrap_or(0);

    // Helper: create GraphEdge + InfraEdge for system connections
    let insert_infra = |from_node: u64, to_node: u64, etype: u8, infra: u8, capacity: f32| {
        let ge = ctx.db.graph_edge().insert(GraphEdge {
            id: 0,
            from_node,
            to_node,
            edge_type: etype,
            weight: capacity,
            bidirectional: false,
        });
        ctx.db.infra_edge().insert(InfraEdge {
            id: 0,
            graph_edge_id: ge.id,
            edge_type: infra,
            corridor_id: svc_corridor_id,
            capacity,
            current_flow: capacity,
            health: 1.0,
        });
    };

    // ---- POWER SYSTEM ----
    let power_sys = insert_system(
        "Power System",
        system_types::POWER,
        power_priorities::CRITICAL,
    );

    let reactor_core = insert_subsystem(
        power_sys,
        "Reactor Core",
        subsystem_types::REACTOR_CORE,
        reactor_node,
        0.0,
        2,
    );
    insert_component(
        reactor_core,
        "Primary Fuel Injector",
        component_types::FUEL_INJECTOR,
        -2.0,
        0.0,
        500.0,
    );
    insert_component(
        reactor_core,
        "Secondary Fuel Injector",
        component_types::FUEL_INJECTOR,
        2.0,
        0.0,
        500.0,
    );
    insert_component(
        reactor_core,
        "Containment Coil A",
        component_types::CONTAINMENT_COIL,
        -1.0,
        -2.0,
        1000.0,
    );
    insert_component(
        reactor_core,
        "Containment Coil B",
        component_types::CONTAINMENT_COIL,
        1.0,
        -2.0,
        1000.0,
    );
    insert_component(
        reactor_core,
        "Core Temperature Sensor",
        component_types::SENSOR,
        0.0,
        0.0,
        200.0,
    );

    let fuel_inj = insert_subsystem(
        power_sys,
        "Fuel Injection System",
        subsystem_types::FUEL_INJECTION,
        reactor_node,
        2.0,
        1,
    );
    insert_component(
        fuel_inj,
        "Fuel Pump",
        component_types::PUMP,
        -1.0,
        1.0,
        300.0,
    );
    insert_component(
        fuel_inj,
        "Flow Regulator",
        component_types::REGULATOR,
        1.0,
        1.0,
        400.0,
    );

    let containment = insert_subsystem(
        power_sys,
        "Magnetic Containment",
        subsystem_types::MAGNETIC_CONTAINMENT,
        reactor_node,
        15.0,
        1,
    );
    insert_component(
        containment,
        "Containment Field Generator",
        component_types::GENERATOR,
        0.0,
        -1.0,
        800.0,
    );
    insert_component(
        containment,
        "Field Strength Sensor",
        component_types::SENSOR,
        0.0,
        1.0,
        200.0,
    );

    let reactor_cool = insert_subsystem(
        power_sys,
        "Reactor Cooling",
        subsystem_types::REACTOR_COOLING,
        cooling_node,
        10.0,
        1,
    );
    insert_component(
        reactor_cool,
        "Primary Coolant Pump",
        component_types::PUMP,
        -2.0,
        0.0,
        250.0,
    );
    insert_component(
        reactor_cool,
        "Backup Coolant Pump",
        component_types::PUMP,
        2.0,
        0.0,
        250.0,
    );
    insert_component(
        reactor_cool,
        "Coolant Temperature Sensor",
        component_types::SENSOR,
        0.0,
        0.0,
        150.0,
    );

    let power_bus = insert_subsystem(
        power_sys,
        "Primary Power Bus",
        subsystem_types::PRIMARY_POWER_BUS,
        power_dist_node,
        1.0,
        1,
    );
    insert_component(
        power_bus,
        "Main Transformer",
        component_types::TRANSFORMER,
        -1.0,
        0.0,
        600.0,
    );
    insert_component(
        power_bus,
        "Bus Circuit Breaker",
        component_types::CIRCUIT_BREAKER,
        1.0,
        0.0,
        400.0,
    );

    let deck_dist = insert_subsystem(
        power_sys,
        "Deck Distribution",
        subsystem_types::DECK_DISTRIBUTION,
        power_dist_node,
        1.0,
        1,
    );
    insert_component(
        deck_dist,
        "Distribution Panel",
        component_types::CIRCUIT_BREAKER,
        0.0,
        -1.0,
        350.0,
    );
    insert_component(
        deck_dist,
        "Load Sensor",
        component_types::SENSOR,
        0.0,
        1.0,
        200.0,
    );

    let emerg_bus = insert_subsystem(
        power_sys,
        "Emergency Power Bus",
        subsystem_types::EMERGENCY_BUS,
        power_dist_node,
        0.5,
        1,
    );
    insert_component(
        emerg_bus,
        "Emergency Breaker",
        component_types::CIRCUIT_BREAKER,
        0.0,
        0.0,
        300.0,
    );

    let emerg_gen1 = insert_subsystem(
        power_sys,
        "Emergency Generator 1",
        subsystem_types::EMERGENCY_GENERATOR,
        engineering_node,
        0.0,
        1,
    );
    insert_component(
        emerg_gen1,
        "Generator Motor",
        component_types::GENERATOR,
        0.0,
        0.0,
        500.0,
    );
    let emerg_gen2 = insert_subsystem(
        power_sys,
        "Emergency Generator 2",
        subsystem_types::EMERGENCY_GENERATOR,
        engineering_node,
        0.0,
        1,
    );
    insert_component(
        emerg_gen2,
        "Generator Motor",
        component_types::GENERATOR,
        0.0,
        0.0,
        500.0,
    );

    // ---- LIFE SUPPORT ----
    let ls_sys = insert_system(
        "Life Support",
        system_types::LIFE_SUPPORT,
        power_priorities::CRITICAL,
    );

    let o2_gen = insert_subsystem(
        ls_sys,
        "O2 Generation",
        subsystem_types::O2_GENERATION,
        ls_node,
        20.0,
        1,
    );
    insert_component(
        o2_gen,
        "Electrolysis Cell A",
        component_types::GENERATOR,
        -2.0,
        0.0,
        400.0,
    );
    insert_component(
        o2_gen,
        "Electrolysis Cell B",
        component_types::GENERATOR,
        2.0,
        0.0,
        400.0,
    );
    insert_component(
        o2_gen,
        "O2 Level Sensor",
        component_types::SENSOR,
        0.0,
        1.0,
        150.0,
    );

    let co2_scrub = insert_subsystem(
        ls_sys,
        "CO2 Scrubbers",
        subsystem_types::CO2_SCRUBBING,
        ls_node,
        12.0,
        1,
    );
    insert_component(
        co2_scrub,
        "Scrubber Filter A",
        component_types::FILTER,
        -1.0,
        0.0,
        200.0,
    );
    insert_component(
        co2_scrub,
        "Scrubber Filter B",
        component_types::FILTER,
        1.0,
        0.0,
        200.0,
    );
    insert_component(
        co2_scrub,
        "CO2 Sensor",
        component_types::SENSOR,
        0.0,
        0.0,
        150.0,
    );

    let air_circ = insert_subsystem(
        ls_sys,
        "Air Circulation",
        subsystem_types::AIR_CIRCULATION,
        hvac_node,
        8.0,
        1,
    );
    insert_component(
        air_circ,
        "Circulation Fan A",
        component_types::FAN,
        -1.0,
        0.0,
        300.0,
    );
    insert_component(
        air_circ,
        "Circulation Fan B",
        component_types::FAN,
        1.0,
        0.0,
        300.0,
    );
    insert_component(
        air_circ,
        "Airflow Sensor",
        component_types::SENSOR,
        0.0,
        1.0,
        150.0,
    );

    let heat_ex = insert_subsystem(
        ls_sys,
        "Heat Exchangers",
        subsystem_types::HEAT_EXCHANGE,
        cooling_node,
        6.0,
        1,
    );
    insert_component(
        heat_ex,
        "Heat Exchanger Unit",
        component_types::HEAT_EXCHANGER,
        0.0,
        0.0,
        500.0,
    );
    insert_component(
        heat_ex,
        "Temperature Sensor",
        component_types::SENSOR,
        0.0,
        1.0,
        150.0,
    );

    let coolant_pump = insert_subsystem(
        ls_sys,
        "Coolant Pumps",
        subsystem_types::COOLANT_PUMP,
        cooling_node,
        5.0,
        1,
    );
    insert_component(
        coolant_pump,
        "Main Coolant Pump",
        component_types::PUMP,
        0.0,
        0.0,
        250.0,
    );
    insert_component(
        coolant_pump,
        "Coolant Valve",
        component_types::VALVE,
        1.0,
        0.0,
        300.0,
    );

    let radiator = insert_subsystem(
        ls_sys,
        "Radiator Panels",
        subsystem_types::RADIATOR_PANEL,
        cooling_node,
        0.0,
        0,
    );
    insert_component(
        radiator,
        "Radiator Panel Array",
        component_types::HEAT_EXCHANGER,
        0.0,
        0.0,
        600.0,
    );

    let pressure = insert_subsystem(
        ls_sys,
        "Pressure Management",
        subsystem_types::PRESSURE_MANAGEMENT,
        ls_node,
        3.0,
        1,
    );
    insert_component(
        pressure,
        "Pressure Regulator",
        component_types::REGULATOR,
        0.0,
        -1.0,
        350.0,
    );
    insert_component(
        pressure,
        "Bulkhead Seal Actuator",
        component_types::ACTUATOR,
        0.0,
        1.0,
        400.0,
    );
    insert_component(
        pressure,
        "Pressure Sensor",
        component_types::SENSOR,
        1.0,
        0.0,
        150.0,
    );

    // ---- WATER SYSTEM ----
    let water_sys = insert_system(
        "Water System",
        system_types::WATER_RECYCLING,
        power_priorities::NORMAL,
    );

    let water_filt = insert_subsystem(
        water_sys,
        "Water Filtration",
        subsystem_types::WATER_FILTRATION,
        water_node,
        8.0,
        1,
    );
    insert_component(
        water_filt,
        "Filtration Membrane",
        component_types::FILTER,
        0.0,
        -1.0,
        200.0,
    );
    insert_component(
        water_filt,
        "Sediment Filter",
        component_types::FILTER,
        0.0,
        1.0,
        150.0,
    );

    let water_dist_sub = insert_subsystem(
        water_sys,
        "Water Distillation",
        subsystem_types::WATER_DISTILLATION,
        water_node,
        10.0,
        1,
    );
    insert_component(
        water_dist_sub,
        "Distillation Column",
        component_types::HEAT_EXCHANGER,
        0.0,
        0.0,
        400.0,
    );
    insert_component(
        water_dist_sub,
        "Distillation Heater",
        component_types::GENERATOR,
        1.0,
        0.0,
        350.0,
    );

    let uv_purify = insert_subsystem(
        water_sys,
        "UV Purification",
        subsystem_types::UV_PURIFICATION,
        water_node,
        4.0,
        0,
    );
    insert_component(
        uv_purify,
        "UV Lamp Array",
        component_types::LAMP,
        0.0,
        0.0,
        300.0,
    );
    insert_component(
        uv_purify,
        "UV Sensor",
        component_types::SENSOR,
        0.0,
        1.0,
        150.0,
    );

    let water_store = insert_subsystem(
        water_sys,
        "Water Storage Tanks",
        subsystem_types::WATER_STORAGE,
        water_node,
        1.0,
        0,
    );
    insert_component(
        water_store,
        "Main Tank",
        component_types::TANK,
        -1.0,
        0.0,
        800.0,
    );
    insert_component(
        water_store,
        "Level Sensor",
        component_types::SENSOR,
        1.0,
        0.0,
        200.0,
    );

    let water_pump = insert_subsystem(
        water_sys,
        "Water Distribution",
        subsystem_types::WATER_DISTRIBUTION,
        water_node,
        5.0,
        1,
    );
    insert_component(
        water_pump,
        "Distribution Pump",
        component_types::PUMP,
        0.0,
        0.0,
        250.0,
    );
    insert_component(
        water_pump,
        "Pressure Valve",
        component_types::VALVE,
        1.0,
        0.0,
        300.0,
    );

    let waste_proc = insert_subsystem(
        water_sys,
        "Waste Processing",
        subsystem_types::WASTE_PROCESSING,
        waste_node,
        6.0,
        1,
    );
    insert_component(
        waste_proc,
        "Bioreactor",
        component_types::TANK,
        -1.0,
        0.0,
        500.0,
    );
    insert_component(
        waste_proc,
        "Solids Separator",
        component_types::FILTER,
        1.0,
        0.0,
        300.0,
    );

    // ---- FOOD PRODUCTION ----
    let food_sys = insert_system(
        "Food Production",
        system_types::FOOD_PRODUCTION,
        power_priorities::NORMAL,
    );

    let growth = insert_subsystem(
        food_sys,
        "Growth Chambers",
        subsystem_types::GROWTH_CHAMBER,
        hydro_node,
        12.0,
        2,
    );
    insert_component(
        growth,
        "Grow Bed A",
        component_types::TANK,
        -2.0,
        0.0,
        600.0,
    );
    insert_component(growth, "Grow Bed B", component_types::TANK, 2.0, 0.0, 600.0);
    insert_component(
        growth,
        "Soil Moisture Sensor",
        component_types::SENSOR,
        0.0,
        1.0,
        100.0,
    );

    let nutrients = insert_subsystem(
        food_sys,
        "Nutrient Mixer",
        subsystem_types::NUTRIENT_MIXER,
        hydro_node,
        3.0,
        1,
    );
    insert_component(
        nutrients,
        "Nutrient Pump",
        component_types::PUMP,
        0.0,
        0.0,
        200.0,
    );
    insert_component(
        nutrients,
        "pH Sensor",
        component_types::SENSOR,
        1.0,
        0.0,
        100.0,
    );

    let grow_light = insert_subsystem(
        food_sys,
        "Grow Lighting",
        subsystem_types::GROW_LIGHTING,
        hydro_node,
        15.0,
        0,
    );
    insert_component(
        grow_light,
        "LED Array A",
        component_types::LAMP,
        -1.0,
        0.0,
        400.0,
    );
    insert_component(
        grow_light,
        "LED Array B",
        component_types::LAMP,
        1.0,
        0.0,
        400.0,
    );

    let food_proc = insert_subsystem(
        food_sys,
        "Food Processing",
        subsystem_types::FOOD_PROCESSING,
        galley_node,
        5.0,
        2,
    );
    insert_component(
        food_proc,
        "Processing Unit",
        component_types::MOTOR,
        0.0,
        0.0,
        350.0,
    );

    let cold_store = insert_subsystem(
        food_sys,
        "Cold Storage",
        subsystem_types::COLD_STORAGE,
        galley_node,
        8.0,
        0,
    );
    insert_component(
        cold_store,
        "Refrigeration Compressor",
        component_types::COMPRESSOR,
        0.0,
        0.0,
        400.0,
    );
    insert_component(
        cold_store,
        "Temperature Sensor",
        component_types::SENSOR,
        1.0,
        0.0,
        100.0,
    );

    // ---- PROPULSION ----
    let prop_sys = insert_system(
        "Propulsion",
        system_types::PROPULSION,
        power_priorities::HIGH,
    );

    let thrust = insert_subsystem(
        prop_sys,
        "Thrust Chambers",
        subsystem_types::THRUST_CHAMBER,
        engineering_node,
        0.0,
        2,
    );
    insert_component(
        thrust,
        "Thrust Nozzle A",
        component_types::NOZZLE,
        -2.0,
        0.0,
        700.0,
    );
    insert_component(
        thrust,
        "Thrust Nozzle B",
        component_types::NOZZLE,
        2.0,
        0.0,
        700.0,
    );
    insert_component(
        thrust,
        "Thrust Sensor",
        component_types::SENSOR,
        0.0,
        0.0,
        200.0,
    );

    let fuel_pump_sub = insert_subsystem(
        prop_sys,
        "Fuel Pumps",
        subsystem_types::FUEL_PUMP,
        engineering_node,
        5.0,
        1,
    );
    insert_component(
        fuel_pump_sub,
        "Primary Fuel Pump",
        component_types::PUMP,
        0.0,
        0.0,
        300.0,
    );
    insert_component(
        fuel_pump_sub,
        "Fuel Flow Valve",
        component_types::VALVE,
        1.0,
        0.0,
        300.0,
    );

    let nozzle_act = insert_subsystem(
        prop_sys,
        "Nozzle Actuators",
        subsystem_types::NOZZLE_ACTUATOR,
        engineering_node,
        3.0,
        1,
    );
    insert_component(
        nozzle_act,
        "Gimbal Actuator A",
        component_types::ACTUATOR,
        -1.0,
        0.0,
        400.0,
    );
    insert_component(
        nozzle_act,
        "Gimbal Actuator B",
        component_types::ACTUATOR,
        1.0,
        0.0,
        400.0,
    );

    // ---- NAVIGATION ----
    let nav_sys = insert_system(
        "Navigation",
        system_types::NAVIGATION,
        power_priorities::CRITICAL,
    );

    let star_track = insert_subsystem(
        nav_sys,
        "Star Trackers",
        subsystem_types::STAR_TRACKER,
        bridge_node,
        4.0,
        1,
    );
    insert_component(
        star_track,
        "Star Tracker Camera",
        component_types::SCANNER_HEAD,
        0.0,
        0.0,
        300.0,
    );
    insert_component(
        star_track,
        "Image Processor",
        component_types::PROCESSOR,
        1.0,
        0.0,
        200.0,
    );

    let gyro = insert_subsystem(
        nav_sys,
        "Gyroscopes",
        subsystem_types::GYROSCOPE,
        bridge_node,
        3.0,
        0,
    );
    insert_component(
        gyro,
        "Gyroscope Unit A",
        component_types::MOTOR,
        -1.0,
        0.0,
        500.0,
    );
    insert_component(
        gyro,
        "Gyroscope Unit B",
        component_types::MOTOR,
        1.0,
        0.0,
        500.0,
    );

    let att_thrust = insert_subsystem(
        nav_sys,
        "Attitude Thrusters",
        subsystem_types::ATTITUDE_THRUSTER,
        engineering_node,
        2.0,
        0,
    );
    insert_component(
        att_thrust,
        "Thruster Pack Fore",
        component_types::NOZZLE,
        -1.0,
        0.0,
        400.0,
    );
    insert_component(
        att_thrust,
        "Thruster Pack Aft",
        component_types::NOZZLE,
        1.0,
        0.0,
        400.0,
    );

    // ---- COMMUNICATIONS ----
    let comms_sys = insert_system(
        "Communications",
        system_types::COMMUNICATIONS,
        power_priorities::HIGH,
    );

    let antenna = insert_subsystem(
        comms_sys,
        "Antenna Array",
        subsystem_types::ANTENNA_ARRAY,
        comms_node,
        5.0,
        1,
    );
    insert_component(
        antenna,
        "Primary Antenna",
        component_types::ANTENNA,
        -1.0,
        0.0,
        600.0,
    );
    insert_component(
        antenna,
        "Secondary Antenna",
        component_types::ANTENNA,
        1.0,
        0.0,
        600.0,
    );

    let sig_proc = insert_subsystem(
        comms_sys,
        "Signal Processors",
        subsystem_types::SIGNAL_PROCESSOR,
        comms_node,
        3.0,
        1,
    );
    insert_component(
        sig_proc,
        "Signal Processor Unit",
        component_types::PROCESSOR,
        0.0,
        0.0,
        300.0,
    );
    insert_component(
        sig_proc,
        "Encryption Module",
        component_types::PROCESSOR,
        1.0,
        0.0,
        400.0,
    );

    let intercom = insert_subsystem(
        comms_sys,
        "Intercom Network",
        subsystem_types::INTERCOM_NETWORK,
        comms_node,
        2.0,
        0,
    );
    insert_component(
        intercom,
        "Intercom Hub",
        component_types::PROCESSOR,
        0.0,
        0.0,
        250.0,
    );

    let data_back = insert_subsystem(
        comms_sys,
        "Data Backbone",
        subsystem_types::DATA_BACKBONE,
        comms_node,
        3.0,
        0,
    );
    insert_component(
        data_back,
        "Network Switch A",
        component_types::PROCESSOR,
        -1.0,
        0.0,
        350.0,
    );
    insert_component(
        data_back,
        "Network Switch B",
        component_types::PROCESSOR,
        1.0,
        0.0,
        350.0,
    );

    // ---- GRAVITY ----
    let grav_sys = insert_system(
        "Gravity System",
        system_types::GRAVITY,
        power_priorities::NORMAL,
    );

    let grav_ctrl = insert_subsystem(
        grav_sys,
        "Gravity Controller",
        subsystem_types::GRAVITY_CONTROLLER,
        engineering_node,
        5.0,
        1,
    );
    insert_component(
        grav_ctrl,
        "Central Controller",
        component_types::PROCESSOR,
        0.0,
        0.0,
        400.0,
    );

    let grav_plate = insert_subsystem(
        grav_sys,
        "Gravity Plates",
        subsystem_types::GRAVITY_PLATE,
        engineering_node,
        50.0,
        0,
    );
    insert_component(
        grav_plate,
        "Gravity Emitter Array",
        component_types::GRAVITY_EMITTER,
        0.0,
        0.0,
        800.0,
    );

    let dampener = insert_subsystem(
        grav_sys,
        "Inertial Dampeners",
        subsystem_types::INERTIAL_DAMPENER,
        engineering_node,
        15.0,
        0,
    );
    insert_component(
        dampener,
        "Dampener Compensator",
        component_types::CAPACITOR,
        0.0,
        0.0,
        500.0,
    );

    // ---- MEDICAL ----
    let med_sys = insert_system(
        "Medical Systems",
        system_types::MEDICAL,
        power_priorities::HIGH,
    );

    let diag = insert_subsystem(
        med_sys,
        "Diagnostic Scanner",
        subsystem_types::DIAGNOSTIC_SCANNER,
        medical_node,
        4.0,
        1,
    );
    insert_component(
        diag,
        "Body Scanner",
        component_types::SCANNER_HEAD,
        0.0,
        0.0,
        300.0,
    );
    insert_component(
        diag,
        "Scanner Display",
        component_types::DISPLAY,
        1.0,
        0.0,
        200.0,
    );

    let lab = insert_subsystem(
        med_sys,
        "Lab Analyzer",
        subsystem_types::LAB_ANALYZER,
        medical_node,
        3.0,
        1,
    );
    insert_component(
        lab,
        "Chemical Analyzer",
        component_types::PROCESSOR,
        0.0,
        0.0,
        250.0,
    );

    let surgery_sub = insert_subsystem(
        med_sys,
        "Surgical Suite",
        subsystem_types::SURGICAL_SUITE,
        medical_node,
        8.0,
        2,
    );
    insert_component(
        surgery_sub,
        "Surgical Arm",
        component_types::ACTUATOR,
        -1.0,
        0.0,
        400.0,
    );
    insert_component(
        surgery_sub,
        "Surgical Display",
        component_types::DISPLAY,
        1.0,
        0.0,
        200.0,
    );

    let cryo = insert_subsystem(
        med_sys,
        "Cryo Pods",
        subsystem_types::CRYO_POD,
        medical_node,
        6.0,
        0,
    );
    insert_component(
        cryo,
        "Cryo Pod A",
        component_types::COMPRESSOR,
        -1.0,
        0.0,
        600.0,
    );
    insert_component(
        cryo,
        "Cryo Pod B",
        component_types::COMPRESSOR,
        1.0,
        0.0,
        600.0,
    );

    // ---- INFRASTRUCTURE EDGES (resource flow graph) ----
    // Power flow
    insert_infra(
        reactor_node,
        power_dist_node,
        edge_types::POWER,
        infra_types::POWER_CABLE,
        100.0,
    );
    insert_infra(
        power_dist_node,
        engineering_node,
        edge_types::POWER,
        infra_types::POWER_CABLE,
        90.0,
    );
    insert_infra(
        engineering_node,
        ls_node,
        edge_types::POWER,
        infra_types::POWER_CABLE,
        40.0,
    );
    insert_infra(
        engineering_node,
        cooling_node,
        edge_types::POWER,
        infra_types::POWER_CABLE,
        20.0,
    );
    insert_infra(
        engineering_node,
        hvac_node,
        edge_types::POWER,
        infra_types::POWER_CABLE,
        10.0,
    );
    insert_infra(
        engineering_node,
        water_node,
        edge_types::POWER,
        infra_types::POWER_CABLE,
        15.0,
    );
    insert_infra(
        engineering_node,
        hydro_node,
        edge_types::POWER,
        infra_types::POWER_CABLE,
        15.0,
    );
    insert_infra(
        engineering_node,
        galley_node,
        edge_types::POWER,
        infra_types::POWER_CABLE,
        10.0,
    );
    insert_infra(
        engineering_node,
        bridge_node,
        edge_types::POWER,
        infra_types::POWER_CABLE,
        10.0,
    );
    insert_infra(
        engineering_node,
        comms_node,
        edge_types::POWER,
        infra_types::POWER_CABLE,
        8.0,
    );
    insert_infra(
        engineering_node,
        medical_node,
        edge_types::POWER,
        infra_types::POWER_CABLE,
        12.0,
    );

    // Coolant flow
    insert_infra(
        reactor_node,
        cooling_node,
        edge_types::COOLANT,
        infra_types::COOLANT_PIPE,
        50.0,
    );
    insert_infra(
        cooling_node,
        ls_node,
        edge_types::COOLANT,
        infra_types::COOLANT_PIPE,
        30.0,
    );

    // Water flow
    insert_infra(
        waste_node,
        water_node,
        edge_types::WATER,
        infra_types::WATER_PIPE,
        30.0,
    );
    insert_infra(
        water_node,
        galley_node,
        edge_types::WATER,
        infra_types::WATER_PIPE,
        10.0,
    );
    insert_infra(
        water_node,
        hydro_node,
        edge_types::WATER,
        infra_types::WATER_PIPE,
        10.0,
    );
    insert_infra(
        water_node,
        medical_node,
        edge_types::WATER,
        infra_types::WATER_PIPE,
        5.0,
    );

    // HVAC flow
    insert_infra(
        hvac_node,
        ls_node,
        edge_types::HVAC,
        infra_types::HVAC_DUCT,
        40.0,
    );
    insert_infra(
        hvac_node,
        bridge_node,
        edge_types::HVAC,
        infra_types::HVAC_DUCT,
        10.0,
    );
    insert_infra(
        hvac_node,
        medical_node,
        edge_types::HVAC,
        infra_types::HVAC_DUCT,
        10.0,
    );

    // Data connections
    insert_infra(
        comms_node,
        bridge_node,
        edge_types::DATA,
        infra_types::DATA_CABLE,
        10.0,
    );
    insert_infra(
        comms_node,
        engineering_node,
        edge_types::DATA,
        infra_types::DATA_CABLE,
        10.0,
    );
    insert_infra(
        comms_node,
        medical_node,
        edge_types::DATA,
        infra_types::DATA_CABLE,
        5.0,
    );
}

pub(super) fn generate_atmospheres(ctx: &ReducerContext, deck_count: u32) {
    for deck in 0..deck_count as i32 {
        ctx.db.deck_atmosphere().insert(DeckAtmosphere {
            deck,
            oxygen: 0.21,
            co2: 0.0004,
            humidity: 0.45,
            temperature: 22.0,
            pressure: 101.3,
        });
    }
}
