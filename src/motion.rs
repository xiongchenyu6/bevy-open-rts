//! Procedural 3D RTS unit & commander motion engine: 120 named actions across
//! locomotion, weapon charge/windup, attack/recoil, evasive maneuvers,
//! directional hitstun/stagger, status ailments/buffs, economy/construction,
//! superunit/boss sequences, squad/co-op signals, and victory/death poses.
//!
//! Strictly presentation-only: every pose is applied to child visual rigs
//! relative to their saved rest transforms and never mutates root unit
//! `Transform.translation`, `Health`, or any authoritative simulation state.

use std::f32::consts::{PI, TAU};

use crate::*;

/// High-frequency zero-drift impact micro-tremor frequency (Hz) applied to a
/// struck unit during its local hit-react window (60–65 Hz sweet spot).
pub(crate) const IMPACT_TREMOR_HZ: f32 = 62.5;

/// Default duration (seconds) of the local hitstop/impact micro-tremor window.
pub(crate) const IMPACT_TREMOR_DURATION_SEC: f32 = 0.11;

/// High-level grouping for every action in the RTS motion catalog.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum MotionCategory {
    Locomotion,
    ChargeWindup,
    AttackRecoil,
    EvasiveRecovery,
    HitReaction,
    StatusAilment,
    EconomyUtility,
    SuperunitBoss,
    SquadCoop,
    OutcomeDeath,
}

/// Static metadata describing a single named RTS action.
#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(dead_code)]
pub(crate) struct MotionSpec {
    pub(crate) id: MotionId,
    pub(crate) name_zh: &'static str,
    pub(crate) name_en: &'static str,
    pub(crate) category: MotionCategory,
    /// Higher-priority motions interrupt lower-priority ones (0..=100).
    pub(crate) priority: u8,
    /// Nominal cycle or one-shot duration in seconds (> 0).
    pub(crate) duration: f32,
    pub(crate) looping: bool,
}

/// 120 named RTS unit, vehicle, aircraft, superunit, and commander motions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub(crate) enum MotionId {
    // --- Locomotion & Stances (18) ---
    IdleReady,
    IdleGuard,
    InfantryMarch,
    InfantrySprint,
    SprintSkidStop,
    CrouchCover,
    ProneSniper,
    MechHeavyStride,
    WalkerPivotTurn,
    TankTreadRoll,
    TankBrakeDip,
    SiegeDeployAnchor,
    SiegeUndeployPack,
    HoverBankLeft,
    HoverBankRight,
    AirCruiseHover,
    AirDiveStrafe,
    LowHpLimp,

    // --- Weapon Charge, Windup & Telegraphs (14) ---
    RailgunCharge,
    ArtilleryBrace,
    PlasmaOvercharge,
    EmpCapacitorCharge,
    MortarTubeAim,
    MissileLockOn,
    MeleeLungeWindup,
    BeamFocusSpool,
    TeslaCoilSpool,
    CryoTankPressurize,
    FlamerPilotIgnite,
    SiegeDrillRevUp,
    SniperBreathHold,
    OrbitalDesignatorLock,

    // --- Attack, Firing & Recoil Sequences (18) ---
    RifleBurstFire,
    HeavyMachinegunSpray,
    HeavyCannonKickback,
    TwinAutocannonRecoil,
    SiegeShellBlastRecoil,
    FlameSweepArc,
    CryoStreamSweep,
    SniperSnapRecoil,
    RailgunHyperKick,
    RocketSalvoLaunch,
    MortarLaunchThump,
    BeamChannelSustain,
    TeslaArcDischarge,
    MechStompSlam,
    DrillBoreThrust,
    GrenadeLobThrow,
    FlakSkywardBurst,
    BomberPayloadDrop,

    // --- Dodge, Evasive Maneuvers & Recovery (10) ---
    InfantryCombatRoll,
    SideDashEvade,
    ScatterDive,
    VehicleJukeSwerve,
    AirBarrelRoll,
    FlarePopClimb,
    PhaseBlinkShift,
    ChronoWarpRebound,
    LandingShockAbsorb,
    RecoilStanceRecover,

    // --- Directional Hitstun, Stagger & Knockback (12) ---
    FlinchFront,
    FlinchBack,
    FlinchLeft,
    FlinchRight,
    HeavyArmorRattle,
    ShieldBlockBrace,
    ShieldShatterJolt,
    EmpStunWobble,
    BlastKnockbackTumble,
    CritStaggerReel,
    TurretJamJerk,
    AirTurbulenceJolt,

    // --- Status Ailments & Buffs (12) ---
    EmpDisruptedTremor,
    BurningFlailPanic,
    CryoSlowShiver,
    SuppressedPinnedCrouch,
    ArmorShreddedExposed,
    RepairHealingPulse,
    LowHpSmokeStagger,
    HeroicRallyBuff,
    ChronoHasteOverdrive,
    ShieldAuraHum,
    LowPowerBrownoutSlump,
    VeterancyPromoteSurge,

    // --- Economy, Construction, Harvest & Interact (10) ---
    WorkerHammerBuild,
    WorkerWeldSparks,
    HarvesterDrillOre,
    HarvesterCrystalExtract,
    CargoDumpUnload,
    MineDeployPlant,
    CaptureBeaconChannel,
    SabotageChargePlant,
    MedicFieldTriage,
    RadarScanSweep,

    // --- Commander, Superunit & Boss Motions (10) ---
    SuperunitAwakenRoar,
    SiegeWalkerQuakeStomp,
    AirshipBroadsideRoll,
    SuperweaponChargeFloat,
    BossPhaseOverdrive,
    TitanCannonBrace,
    DreadnoughtBarrageRecoil,
    CommanderTacticalPoint,
    CommanderOrbitalCall,
    BossCoreMeltdown,

    // --- Co-op, Squad Formation, Rally & Social (10) ---
    SquadHoldLineBrace,
    SquadAdvanceWave,
    SquadFormationAlign,
    RallyFlagPlant,
    TacticalPingSignal,
    OfficerSalute,
    SquadVictoryCheer,
    CoopAssistAcknowledge,
    FlankSignalSweep,
    RetreatRegroupWave,

    // --- Victory, Defeat & Death Sequences (6) ---
    VictoryFistPump,
    MatchDefeatSlump,
    InfantryFallBack,
    VehicleCookoffBuck,
    MechCollapseKneel,
    AirSpiralCrash,
}

impl MotionId {
    /// Complete ordered catalog of all 120 RTS motions.
    #[allow(dead_code)]
    pub(crate) const ALL: [MotionId; 120] = [
        // Locomotion & Stances (18)
        Self::IdleReady,
        Self::IdleGuard,
        Self::InfantryMarch,
        Self::InfantrySprint,
        Self::SprintSkidStop,
        Self::CrouchCover,
        Self::ProneSniper,
        Self::MechHeavyStride,
        Self::WalkerPivotTurn,
        Self::TankTreadRoll,
        Self::TankBrakeDip,
        Self::SiegeDeployAnchor,
        Self::SiegeUndeployPack,
        Self::HoverBankLeft,
        Self::HoverBankRight,
        Self::AirCruiseHover,
        Self::AirDiveStrafe,
        Self::LowHpLimp,
        // Weapon Charge, Windup & Telegraphs (14)
        Self::RailgunCharge,
        Self::ArtilleryBrace,
        Self::PlasmaOvercharge,
        Self::EmpCapacitorCharge,
        Self::MortarTubeAim,
        Self::MissileLockOn,
        Self::MeleeLungeWindup,
        Self::BeamFocusSpool,
        Self::TeslaCoilSpool,
        Self::CryoTankPressurize,
        Self::FlamerPilotIgnite,
        Self::SiegeDrillRevUp,
        Self::SniperBreathHold,
        Self::OrbitalDesignatorLock,
        // Attack, Firing & Recoil Sequences (18)
        Self::RifleBurstFire,
        Self::HeavyMachinegunSpray,
        Self::HeavyCannonKickback,
        Self::TwinAutocannonRecoil,
        Self::SiegeShellBlastRecoil,
        Self::FlameSweepArc,
        Self::CryoStreamSweep,
        Self::SniperSnapRecoil,
        Self::RailgunHyperKick,
        Self::RocketSalvoLaunch,
        Self::MortarLaunchThump,
        Self::BeamChannelSustain,
        Self::TeslaArcDischarge,
        Self::MechStompSlam,
        Self::DrillBoreThrust,
        Self::GrenadeLobThrow,
        Self::FlakSkywardBurst,
        Self::BomberPayloadDrop,
        // Dodge, Evasive Maneuvers & Recovery (10)
        Self::InfantryCombatRoll,
        Self::SideDashEvade,
        Self::ScatterDive,
        Self::VehicleJukeSwerve,
        Self::AirBarrelRoll,
        Self::FlarePopClimb,
        Self::PhaseBlinkShift,
        Self::ChronoWarpRebound,
        Self::LandingShockAbsorb,
        Self::RecoilStanceRecover,
        // Directional Hitstun, Stagger & Knockback (12)
        Self::FlinchFront,
        Self::FlinchBack,
        Self::FlinchLeft,
        Self::FlinchRight,
        Self::HeavyArmorRattle,
        Self::ShieldBlockBrace,
        Self::ShieldShatterJolt,
        Self::EmpStunWobble,
        Self::BlastKnockbackTumble,
        Self::CritStaggerReel,
        Self::TurretJamJerk,
        Self::AirTurbulenceJolt,
        // Status Ailments & Buffs (12)
        Self::EmpDisruptedTremor,
        Self::BurningFlailPanic,
        Self::CryoSlowShiver,
        Self::SuppressedPinnedCrouch,
        Self::ArmorShreddedExposed,
        Self::RepairHealingPulse,
        Self::LowHpSmokeStagger,
        Self::HeroicRallyBuff,
        Self::ChronoHasteOverdrive,
        Self::ShieldAuraHum,
        Self::LowPowerBrownoutSlump,
        Self::VeterancyPromoteSurge,
        // Economy, Construction, Harvest & Interact (10)
        Self::WorkerHammerBuild,
        Self::WorkerWeldSparks,
        Self::HarvesterDrillOre,
        Self::HarvesterCrystalExtract,
        Self::CargoDumpUnload,
        Self::MineDeployPlant,
        Self::CaptureBeaconChannel,
        Self::SabotageChargePlant,
        Self::MedicFieldTriage,
        Self::RadarScanSweep,
        // Commander, Superunit & Boss Motions (10)
        Self::SuperunitAwakenRoar,
        Self::SiegeWalkerQuakeStomp,
        Self::AirshipBroadsideRoll,
        Self::SuperweaponChargeFloat,
        Self::BossPhaseOverdrive,
        Self::TitanCannonBrace,
        Self::DreadnoughtBarrageRecoil,
        Self::CommanderTacticalPoint,
        Self::CommanderOrbitalCall,
        Self::BossCoreMeltdown,
        // Co-op, Squad Formation, Rally & Social (10)
        Self::SquadHoldLineBrace,
        Self::SquadAdvanceWave,
        Self::SquadFormationAlign,
        Self::RallyFlagPlant,
        Self::TacticalPingSignal,
        Self::OfficerSalute,
        Self::SquadVictoryCheer,
        Self::CoopAssistAcknowledge,
        Self::FlankSignalSweep,
        Self::RetreatRegroupWave,
        // Victory, Defeat & Death Sequences (6)
        Self::VictoryFistPump,
        Self::MatchDefeatSlump,
        Self::InfantryFallBack,
        Self::VehicleCookoffBuck,
        Self::MechCollapseKneel,
        Self::AirSpiralCrash,
    ];

    /// Look up the full metadata spec for this motion.
    pub(crate) const fn spec(self) -> MotionSpec {
        use MotionCategory as C;
        match self {
            // Locomotion & Stances
            Self::IdleReady => MotionSpec {
                id: self,
                name_zh: "待机戒备",
                name_en: "Idle Ready",
                category: C::Locomotion,
                priority: 5,
                duration: 2.2,
                looping: true,
            },
            Self::IdleGuard => MotionSpec {
                id: self,
                name_zh: "哨卫警戒",
                name_en: "Idle Guard",
                category: C::Locomotion,
                priority: 8,
                duration: 2.6,
                looping: true,
            },
            Self::InfantryMarch => MotionSpec {
                id: self,
                name_zh: "步兵行军",
                name_en: "Infantry March",
                category: C::Locomotion,
                priority: 20,
                duration: 0.65,
                looping: true,
            },
            Self::InfantrySprint => MotionSpec {
                id: self,
                name_zh: "步兵冲刺",
                name_en: "Infantry Sprint",
                category: C::Locomotion,
                priority: 24,
                duration: 0.44,
                looping: true,
            },
            Self::SprintSkidStop => MotionSpec {
                id: self,
                name_zh: "急停滑步",
                name_en: "Sprint Skid Stop",
                category: C::Locomotion,
                priority: 38,
                duration: 0.28,
                looping: false,
            },
            Self::CrouchCover => MotionSpec {
                id: self,
                name_zh: "蹲伏掩护",
                name_en: "Crouch Cover",
                category: C::Locomotion,
                priority: 26,
                duration: 1.4,
                looping: true,
            },
            Self::ProneSniper => MotionSpec {
                id: self,
                name_zh: "卧姿狙击",
                name_en: "Prone Sniper",
                category: C::Locomotion,
                priority: 28,
                duration: 1.8,
                looping: true,
            },
            Self::MechHeavyStride => MotionSpec {
                id: self,
                name_zh: "机甲重步",
                name_en: "Mech Heavy Stride",
                category: C::Locomotion,
                priority: 22,
                duration: 0.85,
                looping: true,
            },
            Self::WalkerPivotTurn => MotionSpec {
                id: self,
                name_zh: "步行机转向",
                name_en: "Walker Pivot Turn",
                category: C::Locomotion,
                priority: 23,
                duration: 0.52,
                looping: true,
            },
            Self::TankTreadRoll => MotionSpec {
                id: self,
                name_zh: "坦克履带推进",
                name_en: "Tank Tread Roll",
                category: C::Locomotion,
                priority: 20,
                duration: 0.6,
                looping: true,
            },
            Self::TankBrakeDip => MotionSpec {
                id: self,
                name_zh: "坦克急刹俯仰",
                name_en: "Tank Brake Dip",
                category: C::Locomotion,
                priority: 39,
                duration: 0.32,
                looping: false,
            },
            Self::SiegeDeployAnchor => MotionSpec {
                id: self,
                name_zh: "攻城架设锚定",
                name_en: "Siege Deploy Anchor",
                category: C::Locomotion,
                priority: 58,
                duration: 0.42,
                looping: false,
            },
            Self::SiegeUndeployPack => MotionSpec {
                id: self,
                name_zh: "攻城收起展开",
                name_en: "Siege Undeploy Pack",
                category: C::Locomotion,
                priority: 56,
                duration: 0.36,
                looping: false,
            },
            Self::HoverBankLeft => MotionSpec {
                id: self,
                name_zh: "悬浮左倾机动",
                name_en: "Hover Bank Left",
                category: C::Locomotion,
                priority: 25,
                duration: 0.75,
                looping: true,
            },
            Self::HoverBankRight => MotionSpec {
                id: self,
                name_zh: "悬浮右倾机动",
                name_en: "Hover Bank Right",
                category: C::Locomotion,
                priority: 25,
                duration: 0.75,
                looping: true,
            },
            Self::AirCruiseHover => MotionSpec {
                id: self,
                name_zh: "空中巡航悬停",
                name_en: "Air Cruise Hover",
                category: C::Locomotion,
                priority: 18,
                duration: 1.6,
                looping: true,
            },
            Self::AirDiveStrafe => MotionSpec {
                id: self,
                name_zh: "低空俯冲扫射",
                name_en: "Air Dive Strafe",
                category: C::Locomotion,
                priority: 34,
                duration: 0.55,
                looping: true,
            },
            Self::LowHpLimp => MotionSpec {
                id: self,
                name_zh: "残血跛行",
                name_en: "Low-HP Limp",
                category: C::Locomotion,
                priority: 32,
                duration: 0.92,
                looping: true,
            },

            // Weapon Charge, Windup & Telegraphs
            Self::RailgunCharge => MotionSpec {
                id: self,
                name_zh: "电磁轨炮蓄能",
                name_en: "Railgun Charge",
                category: C::ChargeWindup,
                priority: 48,
                duration: 0.48,
                looping: false,
            },
            Self::ArtilleryBrace => MotionSpec {
                id: self,
                name_zh: "重炮驻锄蓄力",
                name_en: "Artillery Brace",
                category: C::ChargeWindup,
                priority: 50,
                duration: 0.52,
                looping: false,
            },
            Self::PlasmaOvercharge => MotionSpec {
                id: self,
                name_zh: "等离子过载蓄能",
                name_en: "Plasma Overcharge",
                category: C::ChargeWindup,
                priority: 49,
                duration: 0.44,
                looping: false,
            },
            Self::EmpCapacitorCharge => MotionSpec {
                id: self,
                name_zh: "EMP电容充能",
                name_en: "EMP Capacitor Charge",
                category: C::ChargeWindup,
                priority: 52,
                duration: 0.55,
                looping: false,
            },
            Self::MortarTubeAim => MotionSpec {
                id: self,
                name_zh: "迫击炮校准仰角",
                name_en: "Mortar Tube Aim",
                category: C::ChargeWindup,
                priority: 44,
                duration: 0.45,
                looping: false,
            },
            Self::MissileLockOn => MotionSpec {
                id: self,
                name_zh: "导弹锁定抬架",
                name_en: "Missile Lock-On",
                category: C::ChargeWindup,
                priority: 46,
                duration: 0.4,
                looping: false,
            },
            Self::MeleeLungeWindup => MotionSpec {
                id: self,
                name_zh: "近战突刺前摇",
                name_en: "Melee Lunge Windup",
                category: C::ChargeWindup,
                priority: 47,
                duration: 0.34,
                looping: false,
            },
            Self::BeamFocusSpool => MotionSpec {
                id: self,
                name_zh: "光束聚焦预热",
                name_en: "Beam Focus Spool",
                category: C::ChargeWindup,
                priority: 46,
                duration: 0.42,
                looping: false,
            },
            Self::TeslaCoilSpool => MotionSpec {
                id: self,
                name_zh: "磁暴线圈蓄电",
                name_en: "Tesla Coil Spool",
                category: C::ChargeWindup,
                priority: 48,
                duration: 0.38,
                looping: false,
            },
            Self::CryoTankPressurize => MotionSpec {
                id: self,
                name_zh: "冷冻罐加压预喷",
                name_en: "Cryo Tank Pressurize",
                category: C::ChargeWindup,
                priority: 43,
                duration: 0.36,
                looping: false,
            },
            Self::FlamerPilotIgnite => MotionSpec {
                id: self,
                name_zh: "喷火器点火预燃",
                name_en: "Flamer Pilot Ignite",
                category: C::ChargeWindup,
                priority: 43,
                duration: 0.32,
                looping: false,
            },
            Self::SiegeDrillRevUp => MotionSpec {
                id: self,
                name_zh: "攻城钻头提速",
                name_en: "Siege Drill Rev-Up",
                category: C::ChargeWindup,
                priority: 49,
                duration: 0.4,
                looping: false,
            },
            Self::SniperBreathHold => MotionSpec {
                id: self,
                name_zh: "狙击屏息瞄准",
                name_en: "Sniper Breath Hold",
                category: C::ChargeWindup,
                priority: 45,
                duration: 0.46,
                looping: false,
            },
            Self::OrbitalDesignatorLock => MotionSpec {
                id: self,
                name_zh: "轨道信标锁定",
                name_en: "Orbital Designator Lock",
                category: C::ChargeWindup,
                priority: 62,
                duration: 0.65,
                looping: false,
            },

            // Attack, Firing & Recoil Sequences
            Self::RifleBurstFire => MotionSpec {
                id: self,
                name_zh: "步枪点射后坐",
                name_en: "Rifle Burst Fire",
                category: C::AttackRecoil,
                priority: 54,
                duration: 0.24,
                looping: false,
            },
            Self::HeavyMachinegunSpray => MotionSpec {
                id: self,
                name_zh: "重机枪扫射震颤",
                name_en: "Heavy Machinegun Spray",
                category: C::AttackRecoil,
                priority: 55,
                duration: 0.28,
                looping: false,
            },
            Self::HeavyCannonKickback => MotionSpec {
                id: self,
                name_zh: "重炮发射后坐",
                name_en: "Heavy Cannon Kickback",
                category: C::AttackRecoil,
                priority: 60,
                duration: 0.36,
                looping: false,
            },
            Self::TwinAutocannonRecoil => MotionSpec {
                id: self,
                name_zh: "双联机炮交替后坐",
                name_en: "Twin Autocannon Recoil",
                category: C::AttackRecoil,
                priority: 57,
                duration: 0.28,
                looping: false,
            },
            Self::SiegeShellBlastRecoil => MotionSpec {
                id: self,
                name_zh: "攻城巨炮震退",
                name_en: "Siege Shell Blast Recoil",
                category: C::AttackRecoil,
                priority: 66,
                duration: 0.46,
                looping: false,
            },
            Self::FlameSweepArc => MotionSpec {
                id: self,
                name_zh: "火焰喷射横扫",
                name_en: "Flame Sweep Arc",
                category: C::AttackRecoil,
                priority: 56,
                duration: 0.42,
                looping: false,
            },
            Self::CryoStreamSweep => MotionSpec {
                id: self,
                name_zh: "冷冻喷射扫射",
                name_en: "Cryo Stream Sweep",
                category: C::AttackRecoil,
                priority: 56,
                duration: 0.4,
                looping: false,
            },
            Self::SniperSnapRecoil => MotionSpec {
                id: self,
                name_zh: "狙击瞬发后坐",
                name_en: "Sniper Snap Recoil",
                category: C::AttackRecoil,
                priority: 59,
                duration: 0.3,
                looping: false,
            },
            Self::RailgunHyperKick => MotionSpec {
                id: self,
                name_zh: "电磁炮超高速后坐",
                name_en: "Railgun Hyper Kick",
                category: C::AttackRecoil,
                priority: 64,
                duration: 0.38,
                looping: false,
            },
            Self::RocketSalvoLaunch => MotionSpec {
                id: self,
                name_zh: "火箭齐射顿挫",
                name_en: "Rocket Salvo Launch",
                category: C::AttackRecoil,
                priority: 58,
                duration: 0.34,
                looping: false,
            },
            Self::MortarLaunchThump => MotionSpec {
                id: self,
                name_zh: "迫击炮发射震击",
                name_en: "Mortar Launch Thump",
                category: C::AttackRecoil,
                priority: 60,
                duration: 0.36,
                looping: false,
            },
            Self::BeamChannelSustain => MotionSpec {
                id: self,
                name_zh: "激光持续照射",
                name_en: "Beam Channel Sustain",
                category: C::AttackRecoil,
                priority: 56,
                duration: 0.36,
                looping: false,
            },
            Self::TeslaArcDischarge => MotionSpec {
                id: self,
                name_zh: "磁暴电弧释放",
                name_en: "Tesla Arc Discharge",
                category: C::AttackRecoil,
                priority: 60,
                duration: 0.32,
                looping: false,
            },
            Self::MechStompSlam => MotionSpec {
                id: self,
                name_zh: "机甲重踏震击",
                name_en: "Mech Stomp Slam",
                category: C::AttackRecoil,
                priority: 65,
                duration: 0.44,
                looping: false,
            },
            Self::DrillBoreThrust => MotionSpec {
                id: self,
                name_zh: "钻头推进冲击",
                name_en: "Drill Bore Thrust",
                category: C::AttackRecoil,
                priority: 62,
                duration: 0.34,
                looping: false,
            },
            Self::GrenadeLobThrow => MotionSpec {
                id: self,
                name_zh: "手雷抛投",
                name_en: "Grenade Lob Throw",
                category: C::AttackRecoil,
                priority: 56,
                duration: 0.36,
                looping: false,
            },
            Self::FlakSkywardBurst => MotionSpec {
                id: self,
                name_zh: "防空仰射连发",
                name_en: "Flak Skyward Burst",
                category: C::AttackRecoil,
                priority: 57,
                duration: 0.3,
                looping: false,
            },
            Self::BomberPayloadDrop => MotionSpec {
                id: self,
                name_zh: "轰炸机投弹抬升",
                name_en: "Bomber Payload Drop",
                category: C::AttackRecoil,
                priority: 60,
                duration: 0.4,
                looping: false,
            },

            // Dodge, Evasive Maneuvers & Recovery
            Self::InfantryCombatRoll => MotionSpec {
                id: self,
                name_zh: "步兵战术翻滚",
                name_en: "Infantry Combat Roll",
                category: C::EvasiveRecovery,
                priority: 68,
                duration: 0.42,
                looping: false,
            },
            Self::SideDashEvade => MotionSpec {
                id: self,
                name_zh: "侧向闪避突进",
                name_en: "Side Dash Evade",
                category: C::EvasiveRecovery,
                priority: 66,
                duration: 0.3,
                looping: false,
            },
            Self::ScatterDive => MotionSpec {
                id: self,
                name_zh: "散开卧倒规避",
                name_en: "Scatter Dive",
                category: C::EvasiveRecovery,
                priority: 67,
                duration: 0.38,
                looping: false,
            },
            Self::VehicleJukeSwerve => MotionSpec {
                id: self,
                name_zh: "载具急转规避",
                name_en: "Vehicle Juke Swerve",
                category: C::EvasiveRecovery,
                priority: 64,
                duration: 0.36,
                looping: false,
            },
            Self::AirBarrelRoll => MotionSpec {
                id: self,
                name_zh: "战机桶滚机动",
                name_en: "Air Barrel Roll",
                category: C::EvasiveRecovery,
                priority: 70,
                duration: 0.52,
                looping: false,
            },
            Self::FlarePopClimb => MotionSpec {
                id: self,
                name_zh: "释放干扰弹爬升",
                name_en: "Flare Pop Climb",
                category: C::EvasiveRecovery,
                priority: 65,
                duration: 0.44,
                looping: false,
            },
            Self::PhaseBlinkShift => MotionSpec {
                id: self,
                name_zh: "相位闪烁偏移",
                name_en: "Phase Blink Shift",
                category: C::EvasiveRecovery,
                priority: 72,
                duration: 0.24,
                looping: false,
            },
            Self::ChronoWarpRebound => MotionSpec {
                id: self,
                name_zh: "时空跃迁回弹",
                name_en: "Chrono Warp Rebound",
                category: C::EvasiveRecovery,
                priority: 70,
                duration: 0.34,
                looping: false,
            },
            Self::LandingShockAbsorb => MotionSpec {
                id: self,
                name_zh: "空降落地缓冲",
                name_en: "Landing Shock Absorb",
                category: C::EvasiveRecovery,
                priority: 63,
                duration: 0.38,
                looping: false,
            },
            Self::RecoilStanceRecover => MotionSpec {
                id: self,
                name_zh: "后坐力姿态恢复",
                name_en: "Recoil Stance Recover",
                category: C::EvasiveRecovery,
                priority: 36,
                duration: 0.25,
                looping: false,
            },

            // Directional Hitstun, Stagger & Knockback
            Self::FlinchFront => MotionSpec {
                id: self,
                name_zh: "正面受击仰颤",
                name_en: "Flinch Front",
                category: C::HitReaction,
                priority: 74,
                duration: 0.24,
                looping: false,
            },
            Self::FlinchBack => MotionSpec {
                id: self,
                name_zh: "背后受击前倾",
                name_en: "Flinch Back",
                category: C::HitReaction,
                priority: 74,
                duration: 0.24,
                looping: false,
            },
            Self::FlinchLeft => MotionSpec {
                id: self,
                name_zh: "左侧受击右偏",
                name_en: "Flinch Left",
                category: C::HitReaction,
                priority: 74,
                duration: 0.24,
                looping: false,
            },
            Self::FlinchRight => MotionSpec {
                id: self,
                name_zh: "右侧受击左偏",
                name_en: "Flinch Right",
                category: C::HitReaction,
                priority: 74,
                duration: 0.24,
                looping: false,
            },
            Self::HeavyArmorRattle => MotionSpec {
                id: self,
                name_zh: "重甲受击震颤",
                name_en: "Heavy Armor Rattle",
                category: C::HitReaction,
                priority: 73,
                duration: 0.26,
                looping: false,
            },
            Self::ShieldBlockBrace => MotionSpec {
                id: self,
                name_zh: "护盾格挡承压",
                name_en: "Shield Block Brace",
                category: C::HitReaction,
                priority: 72,
                duration: 0.22,
                looping: false,
            },
            Self::ShieldShatterJolt => MotionSpec {
                id: self,
                name_zh: "破盾失衡震荡",
                name_en: "Shield Shatter Jolt",
                category: C::HitReaction,
                priority: 80,
                duration: 0.36,
                looping: false,
            },
            Self::EmpStunWobble => MotionSpec {
                id: self,
                name_zh: "EMP瘫痪摇晃",
                name_en: "EMP Stun Wobble",
                category: C::HitReaction,
                priority: 86,
                duration: 0.45,
                looping: false,
            },
            Self::BlastKnockbackTumble => MotionSpec {
                id: self,
                name_zh: "爆炸击退翻滚",
                name_en: "Blast Knockback Tumble",
                category: C::HitReaction,
                priority: 82,
                duration: 0.44,
                looping: false,
            },
            Self::CritStaggerReel => MotionSpec {
                id: self,
                name_zh: "暴击重创踉跄",
                name_en: "Crit Stagger Reel",
                category: C::HitReaction,
                priority: 81,
                duration: 0.38,
                looping: false,
            },
            Self::TurretJamJerk => MotionSpec {
                id: self,
                name_zh: "炮塔受震卡顿",
                name_en: "Turret Jam Jerk",
                category: C::HitReaction,
                priority: 75,
                duration: 0.3,
                looping: false,
            },
            Self::AirTurbulenceJolt => MotionSpec {
                id: self,
                name_zh: "防空命中气流颠簸",
                name_en: "Air Turbulence Jolt",
                category: C::HitReaction,
                priority: 76,
                duration: 0.32,
                looping: false,
            },

            // Status Ailments & Buffs
            Self::EmpDisruptedTremor => MotionSpec {
                id: self,
                name_zh: "EMP干扰痉挛",
                name_en: "EMP Disrupted Tremor",
                category: C::StatusAilment,
                priority: 84,
                duration: 0.6,
                looping: true,
            },
            Self::BurningFlailPanic => MotionSpec {
                id: self,
                name_zh: "燃烧剧烈挣扎",
                name_en: "Burning Flail Panic",
                category: C::StatusAilment,
                priority: 42,
                duration: 0.48,
                looping: true,
            },
            Self::CryoSlowShiver => MotionSpec {
                id: self,
                name_zh: "冷冻减速寒颤",
                name_en: "Cryo Slow Shiver",
                category: C::StatusAilment,
                priority: 41,
                duration: 0.56,
                looping: true,
            },
            Self::SuppressedPinnedCrouch => MotionSpec {
                id: self,
                name_zh: "火力压制低伏",
                name_en: "Suppressed Pinned Crouch",
                category: C::StatusAilment,
                priority: 40,
                duration: 0.85,
                looping: true,
            },
            Self::ArmorShreddedExposed => MotionSpec {
                id: self,
                name_zh: "破甲暴露颤抖",
                name_en: "Armor Shredded Exposed",
                category: C::StatusAilment,
                priority: 39,
                duration: 0.72,
                looping: true,
            },
            Self::RepairHealingPulse => MotionSpec {
                id: self,
                name_zh: "维修治疗脉动",
                name_en: "Repair Healing Pulse",
                category: C::StatusAilment,
                priority: 33,
                duration: 0.5,
                looping: false,
            },
            Self::LowHpSmokeStagger => MotionSpec {
                id: self,
                name_zh: "重伤冒烟蹒跚",
                name_en: "Low-HP Smoke Stagger",
                category: C::StatusAilment,
                priority: 37,
                duration: 0.88,
                looping: true,
            },
            Self::HeroicRallyBuff => MotionSpec {
                id: self,
                name_zh: "英雄振奋光环",
                name_en: "Heroic Rally Buff",
                category: C::StatusAilment,
                priority: 35,
                duration: 0.75,
                looping: false,
            },
            Self::ChronoHasteOverdrive => MotionSpec {
                id: self,
                name_zh: "时空加速超频",
                name_en: "Chrono Haste Overdrive",
                category: C::StatusAilment,
                priority: 34,
                duration: 0.42,
                looping: true,
            },
            Self::ShieldAuraHum => MotionSpec {
                id: self,
                name_zh: "护盾力场共振",
                name_en: "Shield Aura Hum",
                category: C::StatusAilment,
                priority: 14,
                duration: 1.4,
                looping: true,
            },
            Self::LowPowerBrownoutSlump => MotionSpec {
                id: self,
                name_zh: "低电欠压低垂",
                name_en: "Low-Power Brownout Slump",
                category: C::StatusAilment,
                priority: 31,
                duration: 1.5,
                looping: true,
            },
            Self::VeterancyPromoteSurge => MotionSpec {
                id: self,
                name_zh: "晋升老兵气场爆发",
                name_en: "Veterancy Promote Surge",
                category: C::StatusAilment,
                priority: 77,
                duration: 0.65,
                looping: false,
            },

            // Economy, Construction, Harvest & Interact
            Self::WorkerHammerBuild => MotionSpec {
                id: self,
                name_zh: "工兵锤击建造",
                name_en: "Worker Hammer Build",
                category: C::EconomyUtility,
                priority: 45,
                duration: 0.56,
                looping: true,
            },
            Self::WorkerWeldSparks => MotionSpec {
                id: self,
                name_zh: "工兵电焊维修",
                name_en: "Worker Weld Sparks",
                category: C::EconomyUtility,
                priority: 44,
                duration: 0.48,
                looping: true,
            },
            Self::HarvesterDrillOre => MotionSpec {
                id: self,
                name_zh: "采矿钻探掘进",
                name_en: "Harvester Drill Ore",
                category: C::EconomyUtility,
                priority: 42,
                duration: 0.44,
                looping: true,
            },
            Self::HarvesterCrystalExtract => MotionSpec {
                id: self,
                name_zh: "晶体光束采集",
                name_en: "Harvester Crystal Extract",
                category: C::EconomyUtility,
                priority: 42,
                duration: 0.68,
                looping: true,
            },
            Self::CargoDumpUnload => MotionSpec {
                id: self,
                name_zh: "矿车卸载倾倒",
                name_en: "Cargo Dump Unload",
                category: C::EconomyUtility,
                priority: 46,
                duration: 0.54,
                looping: false,
            },
            Self::MineDeployPlant => MotionSpec {
                id: self,
                name_zh: "布雷下压埋设",
                name_en: "Mine Deploy Plant",
                category: C::EconomyUtility,
                priority: 49,
                duration: 0.36,
                looping: false,
            },
            Self::CaptureBeaconChannel => MotionSpec {
                id: self,
                name_zh: "占领信标引导",
                name_en: "Capture Beacon Channel",
                category: C::EconomyUtility,
                priority: 47,
                duration: 0.9,
                looping: true,
            },
            Self::SabotageChargePlant => MotionSpec {
                id: self,
                name_zh: "破坏炸药安放",
                name_en: "Sabotage Charge Plant",
                category: C::EconomyUtility,
                priority: 52,
                duration: 0.48,
                looping: false,
            },
            Self::MedicFieldTriage => MotionSpec {
                id: self,
                name_zh: "医疗兵战场急救",
                name_en: "Medic Field Triage",
                category: C::EconomyUtility,
                priority: 46,
                duration: 0.72,
                looping: true,
            },
            Self::RadarScanSweep => MotionSpec {
                id: self,
                name_zh: "雷达扫描旋转",
                name_en: "Radar Scan Sweep",
                category: C::EconomyUtility,
                priority: 16,
                duration: 1.8,
                looping: true,
            },

            // Commander, Superunit & Boss Motions
            Self::SuperunitAwakenRoar => MotionSpec {
                id: self,
                name_zh: "终极兵器启动咆哮",
                name_en: "Superunit Awaken Roar",
                category: C::SuperunitBoss,
                priority: 88,
                duration: 0.95,
                looping: false,
            },
            Self::SiegeWalkerQuakeStomp => MotionSpec {
                id: self,
                name_zh: "重型步行机震地",
                name_en: "Siege Walker Quake Stomp",
                category: C::SuperunitBoss,
                priority: 85,
                duration: 0.58,
                looping: false,
            },
            Self::AirshipBroadsideRoll => MotionSpec {
                id: self,
                name_zh: "空艇舷侧齐射横摇",
                name_en: "Airship Broadside Roll",
                category: C::SuperunitBoss,
                priority: 78,
                duration: 0.62,
                looping: false,
            },
            Self::SuperweaponChargeFloat => MotionSpec {
                id: self,
                name_zh: "超武核心悬浮充能",
                name_en: "Superweapon Charge Float",
                category: C::SuperunitBoss,
                priority: 87,
                duration: 1.1,
                looping: true,
            },
            Self::BossPhaseOverdrive => MotionSpec {
                id: self,
                name_zh: "Boss阶段超载爆发",
                name_en: "Boss Phase Overdrive",
                category: C::SuperunitBoss,
                priority: 92,
                duration: 0.85,
                looping: false,
            },
            Self::TitanCannonBrace => MotionSpec {
                id: self,
                name_zh: "泰坦主炮稳态架设",
                name_en: "Titan Cannon Brace",
                category: C::SuperunitBoss,
                priority: 79,
                duration: 0.56,
                looping: false,
            },
            Self::DreadnoughtBarrageRecoil => MotionSpec {
                id: self,
                name_zh: "无畏舰齐射后坐",
                name_en: "Dreadnought Barrage Recoil",
                category: C::SuperunitBoss,
                priority: 83,
                duration: 0.52,
                looping: false,
            },
            Self::CommanderTacticalPoint => MotionSpec {
                id: self,
                name_zh: "指挥官战术前指",
                name_en: "Commander Tactical Point",
                category: C::SuperunitBoss,
                priority: 30,
                duration: 0.72,
                looping: false,
            },
            Self::CommanderOrbitalCall => MotionSpec {
                id: self,
                name_zh: "指挥官呼叫轨道打击",
                name_en: "Commander Orbital Call",
                category: C::SuperunitBoss,
                priority: 86,
                duration: 0.82,
                looping: false,
            },
            Self::BossCoreMeltdown => MotionSpec {
                id: self,
                name_zh: "Boss核心过热熔毁",
                name_en: "Boss Core Meltdown",
                category: C::SuperunitBoss,
                priority: 96,
                duration: 1.2,
                looping: false,
            },

            // Co-op, Squad Formation, Rally & Social
            Self::SquadHoldLineBrace => MotionSpec {
                id: self,
                name_zh: "小队固守阵线",
                name_en: "Squad Hold Line Brace",
                category: C::SquadCoop,
                priority: 27,
                duration: 1.35,
                looping: true,
            },
            Self::SquadAdvanceWave => MotionSpec {
                id: self,
                name_zh: "小队推进手势",
                name_en: "Squad Advance Wave",
                category: C::SquadCoop,
                priority: 33,
                duration: 0.52,
                looping: false,
            },
            Self::SquadFormationAlign => MotionSpec {
                id: self,
                name_zh: "阵型列队微调",
                name_en: "Squad Formation Align",
                category: C::SquadCoop,
                priority: 29,
                duration: 0.45,
                looping: false,
            },
            Self::RallyFlagPlant => MotionSpec {
                id: self,
                name_zh: "集结点插旗",
                name_en: "Rally Flag Plant",
                category: C::SquadCoop,
                priority: 51,
                duration: 0.58,
                looping: false,
            },
            Self::TacticalPingSignal => MotionSpec {
                id: self,
                name_zh: "战术标记信号",
                name_en: "Tactical Ping Signal",
                category: C::SquadCoop,
                priority: 36,
                duration: 0.46,
                looping: false,
            },
            Self::OfficerSalute => MotionSpec {
                id: self,
                name_zh: "军官敬礼",
                name_en: "Officer Salute",
                category: C::SquadCoop,
                priority: 35,
                duration: 0.78,
                looping: false,
            },
            Self::SquadVictoryCheer => MotionSpec {
                id: self,
                name_zh: "小队胜利欢呼",
                name_en: "Squad Victory Cheer",
                category: C::SquadCoop,
                priority: 90,
                duration: 0.9,
                looping: true,
            },
            Self::CoopAssistAcknowledge => MotionSpec {
                id: self,
                name_zh: "协同支援确认",
                name_en: "Co-op Assist Acknowledge",
                category: C::SquadCoop,
                priority: 37,
                duration: 0.5,
                looping: false,
            },
            Self::FlankSignalSweep => MotionSpec {
                id: self,
                name_zh: "侧翼包抄手势",
                name_en: "Flank Signal Sweep",
                category: C::SquadCoop,
                priority: 34,
                duration: 0.48,
                looping: false,
            },
            Self::RetreatRegroupWave => MotionSpec {
                id: self,
                name_zh: "撤退重整挥手",
                name_en: "Retreat Regroup Wave",
                category: C::SquadCoop,
                priority: 38,
                duration: 0.56,
                looping: false,
            },

            // Victory, Defeat & Death Sequences
            Self::VictoryFistPump => MotionSpec {
                id: self,
                name_zh: "胜利振臂欢呼",
                name_en: "Victory Fist Pump",
                category: C::OutcomeDeath,
                priority: 94,
                duration: 0.82,
                looping: true,
            },
            Self::MatchDefeatSlump => MotionSpec {
                id: self,
                name_zh: "战败低头垂肩",
                name_en: "Match Defeat Slump",
                category: C::OutcomeDeath,
                priority: 94,
                duration: 1.4,
                looping: true,
            },
            Self::InfantryFallBack => MotionSpec {
                id: self,
                name_zh: "步兵中弹倒伏",
                name_en: "Infantry Fall Back",
                category: C::OutcomeDeath,
                priority: 100,
                duration: 0.62,
                looping: false,
            },
            Self::VehicleCookoffBuck => MotionSpec {
                id: self,
                name_zh: "载具殉爆剧震",
                name_en: "Vehicle Cookoff Buck",
                category: C::OutcomeDeath,
                priority: 100,
                duration: 0.55,
                looping: false,
            },
            Self::MechCollapseKneel => MotionSpec {
                id: self,
                name_zh: "机甲失能跪倒",
                name_en: "Mech Collapse Kneel",
                category: C::OutcomeDeath,
                priority: 100,
                duration: 0.78,
                looping: false,
            },
            Self::AirSpiralCrash => MotionSpec {
                id: self,
                name_zh: "战机螺旋坠毁",
                name_en: "Air Spiral Crash",
                category: C::OutcomeDeath,
                priority: 100,
                duration: 0.85,
                looping: false,
            },
        }
    }
}

/// Evaluated procedural pose in the unit's local visual coordinate frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MotionPose {
    /// Local visual offset (units in root-local space). Forward is -Z, up is +Y.
    pub(crate) offset: Vec3,
    /// Local Euler angles (pitch X, yaw Y, roll Z) in radians.
    pub(crate) euler: Vec3,
    /// Multiplicative squash/stretch scale around `Vec3::ONE`.
    pub(crate) scale_mul: Vec3,
}

impl Default for MotionPose {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl MotionPose {
    pub(crate) const IDENTITY: Self = Self {
        offset: Vec3::ZERO,
        euler: Vec3::ZERO,
        scale_mul: Vec3::ONE,
    };

    pub(crate) fn rotation(self) -> Quat {
        Quat::from_euler(EulerRot::YXZ, self.euler.y, self.euler.x, self.euler.z)
    }

    pub(crate) fn sanitized(self) -> Self {
        Self {
            offset: Vec3::new(
                self.offset.x.clamp(-0.45, 0.45),
                self.offset.y.clamp(-0.32, 0.45),
                self.offset.z.clamp(-0.45, 0.45),
            ),
            euler: Vec3::new(
                self.euler.x.clamp(-1.1, 1.1),
                self.euler.y.clamp(-1.1, 1.1),
                self.euler.z.clamp(-1.1, 1.1),
            ),
            scale_mul: Vec3::new(
                self.scale_mul.x.clamp(0.76, 1.28),
                self.scale_mul.y.clamp(0.74, 1.28),
                self.scale_mul.z.clamp(0.76, 1.28),
            ),
        }
    }
}

/// Non-linear AAA anticipation -> explosive strike -> overshoot recoil curve.
/// Input `u` in `0.0..=1.0`.
/// - `0.0..0.18`: negative pull-back anticipation (-0.28)
/// - `0.18..0.34`: explosive snap-through to +1.0 peak
/// - `0.34..1.0`: underdamped overshoot settle back to 0.0
pub(crate) fn anticipation_snap_recoil(u: f32) -> f32 {
    let u = u.clamp(0.0, 1.0);
    if u < 0.18 {
        let t = u / 0.18;
        -0.28 * (t * PI).sin()
    } else if u < 0.34 {
        let t = (u - 0.18) / 0.16;
        let snap = 1.0 - (1.0 - t).powi(3);
        -0.12 * (1.0 - snap) + snap
    } else {
        let t = (u - 0.34) / 0.66;
        (-4.8 * t).exp() * (t * PI * 1.65).cos()
    }
}

/// Smooth critically-damped step for a Vec3 value and its velocity.
pub(crate) fn critically_damped_vec3(
    current: Vec3,
    target: Vec3,
    velocity: &mut Vec3,
    omega: f32,
    dt: f32,
) -> Vec3 {
    let dt = dt.clamp(0.0, 0.1);
    if dt <= 0.0 {
        return current;
    }
    let f = 1.0 + omega * dt;
    let inv_f = 1.0 / (f * f);
    let delta = current - target;
    let next_vel = (*velocity - delta * (omega * omega * dt)) * inv_f;
    let next_pos = target + (delta + (*velocity + delta * omega) * dt) * (-omega * dt).exp();
    *velocity = next_vel;
    next_pos
}

/// Computes the 62.5Hz zero-drift impact micro-tremor offset + squash for a
/// struck unit during its local hit-react window.
pub(crate) fn evaluate_impact_tremor(
    remaining: f32,
    duration: f32,
    intensity: f32,
    local_dir: Vec3,
    clock: f32,
) -> MotionPose {
    if remaining <= 0.0 || duration <= 0.0 || intensity <= 0.0 {
        return MotionPose::IDENTITY;
    }
    let ratio = (remaining / duration).clamp(0.0, 1.0);
    // Smooth envelope reaching exact zero at expiry.
    let env = ratio * ratio * intensity.clamp(0.0, 1.5);
    let phase = clock * IMPACT_TREMOR_HZ * TAU;
    let dir = Vec3::new(local_dir.x, 0.0, local_dir.z).normalize_or(Vec3::Z);
    let side = Vec3::new(-dir.z, 0.0, dir.x);
    let jitter = (dir * phase.sin() + side * (phase * 1.13 + 1.1).cos() * 0.65) * (0.048 * env);
    let squash_y = 1.0 - 0.16 * env;
    let bulge_xz = 1.0 / squash_y.sqrt();
    MotionPose {
        offset: jitter,
        euler: Vec3::new(-dir.z * 0.11 * env, 0.0, dir.x * 0.11 * env),
        scale_mul: Vec3::new(bulge_xz, squash_y, bulge_xz),
    }
    .sanitized()
}

/// Evaluates the procedural target pose for any of the 120 `MotionId`s at
/// normalized progress `u` (`0.0..=1.0`), elapsed `elapsed` seconds, and
/// deterministic per-entity `seed`.
pub(crate) fn evaluate_motion_pose(id: MotionId, u: f32, elapsed: f32, seed: f32) -> MotionPose {
    let u = u.clamp(0.0, 1.0);
    let cyc = u * TAU + seed;
    let wave = cyc.sin();
    let cos_w = cyc.cos();
    let punch = anticipation_snap_recoil(u);
    let bell = (u * PI).sin();
    let decay = (1.0 - u) * (1.0 - u);

    let pose = match id {
        // --- Locomotion & Stances ---
        MotionId::IdleReady => MotionPose {
            offset: Vec3::new(0.0, wave * 0.012, 0.0),
            euler: Vec3::new(wave * 0.014, cos_w * 0.018, 0.0),
            scale_mul: Vec3::new(1.0 - wave * 0.008, 1.0 + wave * 0.012, 1.0 - wave * 0.008),
        },
        MotionId::IdleGuard => MotionPose {
            offset: Vec3::new(0.0, -0.015 + wave * 0.008, 0.0),
            euler: Vec3::new(0.04, (u * TAU).sin() * 0.11, 0.0),
            scale_mul: Vec3::new(1.02, 0.98, 1.02),
        },
        MotionId::InfantryMarch => MotionPose {
            offset: Vec3::new(cos_w * 0.018, (cyc * 2.0).sin().abs() * 0.036, 0.0),
            euler: Vec3::new(0.11 + wave * 0.03, wave * 0.05, cos_w * 0.05),
            scale_mul: Vec3::ONE,
        },
        MotionId::InfantrySprint => MotionPose {
            offset: Vec3::new(cos_w * 0.024, (cyc * 2.0).sin().abs() * 0.055, -0.025),
            euler: Vec3::new(0.24 + wave * 0.05, wave * 0.08, cos_w * 0.085),
            scale_mul: Vec3::new(0.96, 1.05, 1.02),
        },
        MotionId::SprintSkidStop => MotionPose {
            offset: Vec3::new(0.0, -0.05 * bell, -0.09 * decay),
            euler: Vec3::new(-0.26 * bell, 0.12 * bell, -0.1 * bell),
            scale_mul: Vec3::new(
                1.08 * bell + (1.0 - bell),
                1.0 - 0.12 * bell,
                1.06 * bell + (1.0 - bell),
            ),
        },
        MotionId::CrouchCover => MotionPose {
            offset: Vec3::new(0.0, -0.09 + wave * 0.006, 0.02),
            euler: Vec3::new(0.22, 0.06 * wave, 0.04),
            scale_mul: Vec3::new(1.07, 0.84, 1.07),
        },
        MotionId::ProneSniper => MotionPose {
            offset: Vec3::new(0.0, -0.14, 0.04),
            euler: Vec3::new(0.38, wave * 0.015, 0.0),
            scale_mul: Vec3::new(1.12, 0.76, 1.14),
        },
        MotionId::MechHeavyStride => MotionPose {
            offset: Vec3::new(wave * 0.032, (cyc * 2.0).sin().abs() * 0.05 - 0.015, 0.0),
            euler: Vec3::new(0.08 + (cyc * 2.0).cos() * 0.045, wave * 0.07, wave * 0.095),
            scale_mul: Vec3::new(
                1.0 + (cyc * 2.0).cos() * 0.02,
                1.0 - (cyc * 2.0).cos() * 0.03,
                1.0,
            ),
        },
        MotionId::WalkerPivotTurn => MotionPose {
            offset: Vec3::new(0.0, (cyc * 2.0).sin().abs() * 0.026, 0.0),
            euler: Vec3::new(0.03, wave * 0.16, cos_w * 0.08),
            scale_mul: Vec3::ONE,
        },
        MotionId::TankTreadRoll => MotionPose {
            offset: Vec3::new(0.0, (elapsed * 22.0 + seed).sin() * 0.012, 0.0),
            euler: Vec3::new(
                0.045 + (elapsed * 14.0 + seed).sin() * 0.018,
                0.0,
                (elapsed * 11.0 + seed).cos() * 0.02,
            ),
            scale_mul: Vec3::ONE,
        },
        MotionId::TankBrakeDip => MotionPose {
            offset: Vec3::new(0.0, -0.045 * bell, -0.055 * decay),
            euler: Vec3::new(0.22 * punch, 0.0, 0.03 * bell),
            scale_mul: Vec3::new(
                1.05 * bell + (1.0 - bell),
                1.0 - 0.08 * bell,
                1.04 * bell + (1.0 - bell),
            ),
        },
        MotionId::SiegeDeployAnchor => MotionPose {
            offset: Vec3::new(0.0, -0.08 * u.min(1.0) - 0.03 * bell, 0.03 * bell),
            euler: Vec3::new(-0.14 * bell, 0.0, (u * PI * 3.0).sin() * 0.035 * decay),
            scale_mul: Vec3::new(1.0 + 0.1 * u, 1.0 - 0.11 * u, 1.0 + 0.1 * u),
        },
        MotionId::SiegeUndeployPack => MotionPose {
            offset: Vec3::new(0.0, 0.045 * bell, -0.02 * bell),
            euler: Vec3::new(0.11 * bell, 0.0, 0.0),
            scale_mul: Vec3::new(1.0 - 0.05 * bell, 1.0 + 0.08 * bell, 1.0 - 0.05 * bell),
        },
        MotionId::HoverBankLeft => MotionPose {
            offset: Vec3::new(-0.04, wave * 0.035, 0.0),
            euler: Vec3::new(0.1, 0.04, 0.3),
            scale_mul: Vec3::ONE,
        },
        MotionId::HoverBankRight => MotionPose {
            offset: Vec3::new(0.04, wave * 0.035, 0.0),
            euler: Vec3::new(0.1, -0.04, -0.3),
            scale_mul: Vec3::ONE,
        },
        MotionId::AirCruiseHover => MotionPose {
            offset: Vec3::new(cos_w * 0.025, wave * 0.045, 0.0),
            euler: Vec3::new(0.06 + wave * 0.03, 0.0, cos_w * 0.06),
            scale_mul: Vec3::ONE,
        },
        MotionId::AirDiveStrafe => MotionPose {
            offset: Vec3::new(0.0, -0.06 * bell, -0.06 * bell),
            euler: Vec3::new(0.28, wave * 0.06, cos_w * 0.14),
            scale_mul: Vec3::new(0.96, 0.96, 1.08),
        },
        MotionId::LowHpLimp => MotionPose {
            offset: Vec3::new(wave * 0.045, -0.045 + (cyc * 2.0).sin().abs() * 0.022, 0.0),
            euler: Vec3::new(0.18 + wave * 0.06, -0.08, 0.2 + wave * 0.12),
            scale_mul: Vec3::new(1.03, 0.92, 1.03),
        },

        // --- Weapon Charge, Windup & Telegraphs ---
        MotionId::RailgunCharge => {
            let hum = (elapsed * 48.0 + seed).sin() * 0.014 * u;
            MotionPose {
                offset: Vec3::new(hum, -0.03 * u, 0.09 * u),
                euler: Vec3::new(-0.08 * u, 0.0, hum * 2.2),
                scale_mul: Vec3::new(1.0 + 0.09 * u, 1.0 - 0.08 * u, 1.0 - 0.11 * u),
            }
        }
        MotionId::ArtilleryBrace => MotionPose {
            offset: Vec3::new(0.0, -0.075 * u, 0.06 * u),
            euler: Vec3::new(-0.2 * u, 0.0, 0.0),
            scale_mul: Vec3::new(1.0 + 0.11 * u, 1.0 - 0.12 * u, 1.0 + 0.08 * u),
        },
        MotionId::PlasmaOvercharge => {
            let pulse = (elapsed * 34.0 + seed).sin() * 0.04 * u;
            MotionPose {
                offset: Vec3::new(pulse * 0.4, 0.035 * u, 0.04 * u),
                euler: Vec3::new(-0.12 * u, pulse * 1.4, -pulse * 1.6),
                scale_mul: Vec3::splat(1.0 + 0.11 * u + pulse),
            }
        }
        MotionId::EmpCapacitorCharge => {
            let buzz = (elapsed * 56.0 + seed).sin() * 0.02 * u;
            MotionPose {
                offset: Vec3::new(buzz, 0.05 * u, 0.0),
                euler: Vec3::new(0.0, buzz * 3.0, buzz * 2.5),
                scale_mul: Vec3::new(1.0 - 0.07 * u, 1.0 + 0.14 * u, 1.0 - 0.07 * u),
            }
        }
        MotionId::MortarTubeAim => MotionPose {
            offset: Vec3::new(0.0, -0.05 * u, 0.03 * u),
            euler: Vec3::new(-0.28 * u, 0.0, 0.0),
            scale_mul: Vec3::new(1.04 * u + (1.0 - u), 1.0 - 0.06 * u, 1.04 * u + (1.0 - u)),
        },
        MotionId::MissileLockOn => MotionPose {
            offset: Vec3::new(0.0, 0.025 * u, 0.045 * u),
            euler: Vec3::new(-0.19 * u, (u * PI * 2.0).sin() * 0.05, 0.0),
            scale_mul: Vec3::new(1.0, 1.0 + 0.06 * u, 1.0 - 0.04 * u),
        },
        MotionId::MeleeLungeWindup => MotionPose {
            offset: Vec3::new(0.0, -0.06 * u, 0.12 * u),
            euler: Vec3::new(-0.22 * u, -0.24 * u, -0.1 * u),
            scale_mul: Vec3::new(1.08 * u + (1.0 - u), 1.0 - 0.12 * u, 1.0 - 0.08 * u),
        },
        MotionId::BeamFocusSpool => {
            let hum = (elapsed * 42.0 + seed).sin() * 0.01 * u;
            MotionPose {
                offset: Vec3::new(hum, 0.02 * u, -0.03 * u),
                euler: Vec3::new(0.06 * u, 0.0, hum * 2.0),
                scale_mul: Vec3::new(1.0 - 0.06 * u, 1.0 - 0.04 * u, 1.0 + 0.12 * u),
            }
        }
        MotionId::TeslaCoilSpool => {
            let arc = (elapsed * 60.0 + seed).sin() * 0.022 * u;
            MotionPose {
                offset: Vec3::new(arc, 0.04 * u, -arc * 0.5),
                euler: Vec3::new(-0.08 * u, arc * 2.0, -arc * 2.2),
                scale_mul: Vec3::new(1.0 + 0.05 * u, 1.0 + 0.1 * u, 1.0 + 0.05 * u),
            }
        }
        MotionId::CryoTankPressurize => MotionPose {
            offset: Vec3::new(0.0, -0.03 * u, 0.04 * u),
            euler: Vec3::new(0.09 * u, 0.0, (elapsed * 32.0 + seed).sin() * 0.04 * u),
            scale_mul: Vec3::new(1.0 + 0.08 * u, 1.0 - 0.05 * u, 1.0 + 0.08 * u),
        },
        MotionId::FlamerPilotIgnite => MotionPose {
            offset: Vec3::new(0.0, -0.025 * u, 0.05 * u),
            euler: Vec3::new(0.12 * u, -0.14 * u, 0.05 * u),
            scale_mul: Vec3::new(1.04 * u + (1.0 - u), 1.0 - 0.04 * u, 1.05 * u + (1.0 - u)),
        },
        MotionId::SiegeDrillRevUp => {
            let vib = (elapsed * 54.0 + seed).sin() * 0.018 * u;
            MotionPose {
                offset: Vec3::new(vib, -0.03 * u, 0.07 * u),
                euler: Vec3::new(0.1 * u, 0.0, vib * 3.0),
                scale_mul: Vec3::new(1.06 * u + (1.0 - u), 1.0 - 0.07 * u, 1.0 + 0.06 * u),
            }
        }
        MotionId::SniperBreathHold => MotionPose {
            offset: Vec3::new(0.0, -0.055 * u, 0.025 * u),
            euler: Vec3::new(0.14 * u, 0.08 * u, 0.0),
            scale_mul: Vec3::new(0.98, 0.94, 1.04),
        },
        MotionId::OrbitalDesignatorLock => MotionPose {
            offset: Vec3::new(0.0, 0.06 * u, -0.02 * u),
            euler: Vec3::new(-0.24 * u, (u * PI).sin() * 0.12, 0.0),
            scale_mul: Vec3::new(0.96, 1.09 * u + (1.0 - u), 0.96),
        },

        // --- Attack, Firing & Recoil Sequences ---
        MotionId::RifleBurstFire => MotionPose {
            offset: Vec3::new(0.0, 0.012 * punch, 0.075 * punch),
            euler: Vec3::new(-0.14 * punch, 0.04 * punch, 0.0),
            scale_mul: Vec3::new(1.0 + 0.04 * punch, 1.0 + 0.02 * punch, 1.0 - 0.06 * punch),
        },
        MotionId::HeavyMachinegunSpray => {
            let rattle = (elapsed * 46.0 + seed).sin();
            MotionPose {
                offset: Vec3::new(
                    rattle * 0.02 * bell,
                    -0.02 * bell,
                    0.085 * bell + rattle * 0.02,
                ),
                euler: Vec3::new(-0.16 * bell, rattle * 0.07 * bell, rattle * 0.04 * bell),
                scale_mul: Vec3::new(
                    1.05 * bell + (1.0 - bell),
                    1.0 - 0.04 * bell,
                    1.0 - 0.06 * bell,
                ),
            }
        }
        MotionId::HeavyCannonKickback => MotionPose {
            offset: Vec3::new(0.0, -0.035 * punch, 0.18 * punch),
            euler: Vec3::new(-0.26 * punch, 0.0, 0.03 * punch),
            scale_mul: Vec3::new(1.0 + 0.09 * punch, 1.0 - 0.07 * punch, 1.0 - 0.12 * punch),
        },
        MotionId::TwinAutocannonRecoil => {
            let alt = (u * PI * 4.0).sin();
            MotionPose {
                offset: Vec3::new(alt * 0.025 * bell, 0.0, 0.11 * punch),
                euler: Vec3::new(-0.15 * punch, alt * 0.09 * bell, -alt * 0.06 * bell),
                scale_mul: Vec3::new(1.0 + 0.05 * punch, 1.0, 1.0 - 0.08 * punch),
            }
        }
        MotionId::SiegeShellBlastRecoil => MotionPose {
            offset: Vec3::new(0.0, -0.08 * punch, 0.25 * punch),
            euler: Vec3::new(-0.36 * punch, 0.0, (u * PI * 3.0).sin() * 0.05 * decay),
            scale_mul: Vec3::new(1.0 + 0.14 * punch, 1.0 - 0.14 * punch, 1.0 - 0.16 * punch),
        },
        MotionId::FlameSweepArc => MotionPose {
            offset: Vec3::new(0.0, -0.02 * bell, 0.04 * bell),
            euler: Vec3::new(
                0.08 * bell,
                (u * PI * 2.0 - PI).sin() * 0.34 * bell,
                0.06 * bell,
            ),
            scale_mul: Vec3::new(1.04 * bell + (1.0 - bell), 1.0, 1.06 * bell + (1.0 - bell)),
        },
        MotionId::CryoStreamSweep => MotionPose {
            offset: Vec3::new(0.0, -0.015 * bell, 0.03 * bell),
            euler: Vec3::new(
                0.05 * bell,
                -(u * PI * 2.0 - PI).sin() * 0.3 * bell,
                -0.05 * bell,
            ),
            scale_mul: Vec3::new(
                1.02 * bell + (1.0 - bell),
                1.03 * bell + (1.0 - bell),
                1.04 * bell + (1.0 - bell),
            ),
        },
        MotionId::SniperSnapRecoil => MotionPose {
            offset: Vec3::new(0.0, 0.025 * punch, 0.14 * punch),
            euler: Vec3::new(-0.24 * punch, -0.09 * punch, 0.04 * punch),
            scale_mul: Vec3::new(0.96, 1.05 * punch + (1.0 - punch), 1.0 - 0.1 * punch),
        },
        MotionId::RailgunHyperKick => MotionPose {
            offset: Vec3::new(0.0, -0.04 * punch, 0.24 * punch),
            euler: Vec3::new(-0.32 * punch, 0.0, 0.0),
            scale_mul: Vec3::new(1.0 + 0.12 * punch, 1.0 - 0.1 * punch, 1.0 - 0.18 * punch),
        },
        MotionId::RocketSalvoLaunch => {
            let burst = (u * PI * 3.0).sin().abs() * decay;
            MotionPose {
                offset: Vec3::new(0.0, -0.04 * burst, 0.13 * punch),
                euler: Vec3::new(-0.22 * punch, 0.0, (u * PI * 3.0).cos() * 0.06 * decay),
                scale_mul: Vec3::new(1.0 + 0.07 * burst, 1.0 - 0.07 * burst, 1.0 - 0.08 * burst),
            }
        }
        MotionId::MortarLaunchThump => MotionPose {
            offset: Vec3::new(0.0, -0.1 * punch, 0.06 * punch),
            euler: Vec3::new(-0.28 * punch, 0.0, 0.0),
            scale_mul: Vec3::new(1.0 + 0.14 * punch, 1.0 - 0.16 * punch, 1.0 + 0.12 * punch),
        },
        MotionId::BeamChannelSustain => {
            let hum = (elapsed * 52.0 + seed).sin() * 0.012;
            MotionPose {
                offset: Vec3::new(hum, 0.015 * bell, 0.065 * bell),
                euler: Vec3::new(-0.08 * bell, hum * 2.0, -hum * 1.8),
                scale_mul: Vec3::new(1.0 - 0.04 * bell, 1.0 + 0.03 * bell, 1.0 + 0.08 * bell),
            }
        }
        MotionId::TeslaArcDischarge => {
            let zap = (elapsed * 58.0 + seed).sin();
            MotionPose {
                offset: Vec3::new(zap * 0.03 * decay, 0.06 * punch, 0.05 * punch),
                euler: Vec3::new(-0.14 * punch, zap * 0.12 * decay, zap * 0.1 * decay),
                scale_mul: Vec3::new(1.0 + 0.08 * punch, 1.0 + 0.1 * punch, 1.0 + 0.08 * punch),
            }
        }
        MotionId::MechStompSlam => MotionPose {
            offset: Vec3::new(
                0.0,
                0.14 * (1.0 - u) * bell - 0.1 * punch.max(0.0),
                -0.08 * punch,
            ),
            euler: Vec3::new(0.26 * punch, 0.0, 0.08 * punch),
            scale_mul: Vec3::new(
                1.0 + 0.15 * punch.max(0.0),
                1.0 - 0.16 * punch.max(0.0),
                1.0 + 0.14 * punch.max(0.0),
            ),
        },
        MotionId::DrillBoreThrust => MotionPose {
            offset: Vec3::new((elapsed * 50.0).sin() * 0.015, -0.02 * bell, -0.18 * punch),
            euler: Vec3::new(0.16 * punch, 0.0, (elapsed * 40.0).cos() * 0.06 * bell),
            scale_mul: Vec3::new(1.0 - 0.06 * punch, 1.0 - 0.04 * punch, 1.0 + 0.15 * punch),
        },
        MotionId::GrenadeLobThrow => MotionPose {
            offset: Vec3::new(0.0, 0.04 * bell, -0.08 * punch),
            euler: Vec3::new(0.24 * punch, 0.28 * punch, -0.12 * punch),
            scale_mul: Vec3::new(0.96, 1.06 * bell + (1.0 - bell), 1.0),
        },
        MotionId::FlakSkywardBurst => MotionPose {
            offset: Vec3::new(0.0, -0.05 * punch, 0.06 * punch),
            euler: Vec3::new(-0.34 * bell, 0.0, (u * PI * 4.0).sin() * 0.05 * decay),
            scale_mul: Vec3::new(
                1.06 * bell + (1.0 - bell),
                1.0 - 0.08 * bell,
                1.06 * bell + (1.0 - bell),
            ),
        },
        MotionId::BomberPayloadDrop => MotionPose {
            offset: Vec3::new(0.0, 0.12 * bell, -0.04 * bell),
            euler: Vec3::new(-0.18 * bell, 0.0, (u * PI * 2.0).sin() * 0.12),
            scale_mul: Vec3::new(1.04 * bell + (1.0 - bell), 0.96, 1.04 * bell + (1.0 - bell)),
        },

        // --- Dodge, Evasive Maneuvers & Recovery ---
        MotionId::InfantryCombatRoll => MotionPose {
            offset: Vec3::new(0.14 * bell, -0.08 * bell, -0.08 * bell),
            euler: Vec3::new(0.32 * bell, 0.0, -0.78 * bell),
            scale_mul: Vec3::new(
                1.1 * bell + (1.0 - bell),
                1.0 - 0.18 * bell,
                1.08 * bell + (1.0 - bell),
            ),
        },
        MotionId::SideDashEvade => MotionPose {
            offset: Vec3::new(0.18 * bell, 0.02 * bell, 0.0),
            euler: Vec3::new(0.08 * bell, 0.14 * bell, -0.32 * bell),
            scale_mul: Vec3::new(1.12 * bell + (1.0 - bell), 0.94, 0.94),
        },
        MotionId::ScatterDive => MotionPose {
            offset: Vec3::new(-0.12 * bell, -0.14 * bell, -0.12 * bell),
            euler: Vec3::new(0.42 * bell, -0.15 * bell, 0.24 * bell),
            scale_mul: Vec3::new(
                1.12 * bell + (1.0 - bell),
                1.0 - 0.2 * bell,
                1.12 * bell + (1.0 - bell),
            ),
        },
        MotionId::VehicleJukeSwerve => MotionPose {
            offset: Vec3::new(0.11 * bell, 0.015 * bell, 0.0),
            euler: Vec3::new(0.06 * bell, 0.24 * bell, -0.22 * bell),
            scale_mul: Vec3::ONE,
        },
        MotionId::AirBarrelRoll => MotionPose {
            offset: Vec3::new((u * TAU).sin() * 0.12, (u * PI).sin() * 0.1, 0.0),
            euler: Vec3::new(0.08 * bell, 0.0, (u * PI * 2.0).sin() * 0.85),
            scale_mul: Vec3::ONE,
        },
        MotionId::FlarePopClimb => MotionPose {
            offset: Vec3::new(0.0, 0.16 * bell, 0.06 * bell),
            euler: Vec3::new(-0.34 * bell, 0.0, 0.16 * bell),
            scale_mul: Vec3::new(0.96, 1.08 * bell + (1.0 - bell), 0.96),
        },
        MotionId::PhaseBlinkShift => MotionPose {
            offset: Vec3::new(0.0, 0.04 * bell, -0.14 * bell),
            euler: Vec3::new(0.12 * bell, 0.0, 0.0),
            scale_mul: Vec3::new(1.0 - 0.2 * bell, 1.0 + 0.22 * bell, 1.0 + 0.18 * bell),
        },
        MotionId::ChronoWarpRebound => MotionPose {
            offset: Vec3::new(0.0, 0.06 * bell, 0.09 * punch),
            euler: Vec3::new(-0.1 * punch, 0.22 * bell, 0.0),
            scale_mul: Vec3::new(1.0 + 0.14 * bell, 1.0 - 0.12 * bell, 1.0 + 0.14 * bell),
        },
        MotionId::LandingShockAbsorb => MotionPose {
            offset: Vec3::new(0.0, -0.11 * decay, 0.0),
            euler: Vec3::new(0.14 * decay, 0.0, 0.0),
            scale_mul: Vec3::new(1.0 + 0.15 * decay, 1.0 - 0.18 * decay, 1.0 + 0.15 * decay),
        },
        MotionId::RecoilStanceRecover => MotionPose {
            offset: Vec3::new(0.0, -0.02 * decay, 0.05 * decay),
            euler: Vec3::new(-0.08 * decay, 0.0, 0.0),
            scale_mul: Vec3::ONE,
        },

        // --- Directional Hitstun, Stagger & Knockback ---
        MotionId::FlinchFront => MotionPose {
            offset: Vec3::new(0.0, -0.025 * bell, 0.13 * decay),
            euler: Vec3::new(-0.34 * decay, 0.06 * decay, 0.0),
            scale_mul: Vec3::new(1.0 + 0.1 * decay, 1.0 - 0.1 * decay, 1.0 - 0.08 * decay),
        },
        MotionId::FlinchBack => MotionPose {
            offset: Vec3::new(0.0, -0.025 * bell, -0.13 * decay),
            euler: Vec3::new(0.34 * decay, -0.06 * decay, 0.0),
            scale_mul: Vec3::new(1.0 + 0.1 * decay, 1.0 - 0.1 * decay, 1.0 - 0.08 * decay),
        },
        MotionId::FlinchLeft => MotionPose {
            offset: Vec3::new(0.13 * decay, -0.02 * bell, 0.0),
            euler: Vec3::new(0.0, 0.16 * decay, -0.34 * decay),
            scale_mul: Vec3::new(1.0 - 0.1 * decay, 1.0 - 0.08 * decay, 1.0 + 0.12 * decay),
        },
        MotionId::FlinchRight => MotionPose {
            offset: Vec3::new(-0.13 * decay, -0.02 * bell, 0.0),
            euler: Vec3::new(0.0, -0.16 * decay, 0.34 * decay),
            scale_mul: Vec3::new(1.0 - 0.1 * decay, 1.0 - 0.08 * decay, 1.0 + 0.12 * decay),
        },
        MotionId::HeavyArmorRattle => {
            let vib = (elapsed * 58.0 + seed).sin() * decay;
            MotionPose {
                offset: Vec3::new(vib * 0.035, -0.03 * decay, 0.06 * decay),
                euler: Vec3::new(-0.12 * decay, vib * 0.06, vib * 0.09),
                scale_mul: Vec3::new(1.0 + 0.07 * decay, 1.0 - 0.08 * decay, 1.0 + 0.07 * decay),
            }
        }
        MotionId::ShieldBlockBrace => MotionPose {
            offset: Vec3::new(0.0, -0.04 * bell, 0.065 * decay),
            euler: Vec3::new(0.14 * bell, -0.12 * bell, 0.06 * bell),
            scale_mul: Vec3::new(1.0 + 0.08 * decay, 1.0 - 0.09 * decay, 1.0 + 0.04 * decay),
        },
        MotionId::ShieldShatterJolt => {
            let jitter = (elapsed * 48.0 + seed).sin() * decay;
            MotionPose {
                offset: Vec3::new(jitter * 0.04, 0.03 * bell, 0.16 * decay),
                euler: Vec3::new(-0.36 * decay, jitter * 0.18, jitter * 0.22),
                scale_mul: Vec3::new(1.0 - 0.12 * decay, 1.0 + 0.14 * decay, 1.0 - 0.12 * decay),
            }
        }
        MotionId::EmpStunWobble => {
            let zap = (elapsed * 55.0 + seed).sin();
            MotionPose {
                offset: Vec3::new(
                    zap * 0.04 * decay,
                    -0.06 * bell,
                    (elapsed * 41.0).cos() * 0.03 * decay,
                ),
                euler: Vec3::new(
                    0.24 * bell,
                    zap * 0.18 * decay,
                    (u * PI * 3.0).sin() * 0.28 * decay,
                ),
                scale_mul: Vec3::new(1.0 + 0.06 * bell, 1.0 - 0.1 * bell, 1.0 + 0.06 * bell),
            }
        }
        MotionId::BlastKnockbackTumble => MotionPose {
            offset: Vec3::new(0.0, 0.14 * bell, 0.26 * decay),
            euler: Vec3::new(-0.62 * bell, 0.2 * bell, 0.32 * bell),
            scale_mul: Vec3::new(
                1.08 * bell + (1.0 - bell),
                1.0 - 0.14 * bell,
                1.08 * bell + (1.0 - bell),
            ),
        },
        MotionId::CritStaggerReel => MotionPose {
            offset: Vec3::new(
                (u * PI * 2.0).sin() * 0.07 * decay,
                -0.05 * bell,
                0.2 * decay,
            ),
            euler: Vec3::new(-0.44 * decay, -0.22 * decay, 0.24 * decay),
            scale_mul: Vec3::new(1.0 + 0.13 * decay, 1.0 - 0.15 * decay, 1.0 + 0.08 * decay),
        },
        MotionId::TurretJamJerk => {
            let jerk = (u * PI * 5.0).sin() * decay;
            MotionPose {
                offset: Vec3::new(jerk * 0.03, -0.02 * decay, 0.05 * decay),
                euler: Vec3::new(-0.08 * decay, jerk * 0.28, jerk * 0.08),
                scale_mul: Vec3::ONE,
            }
        }
        MotionId::AirTurbulenceJolt => {
            let jolt = (u * PI * 4.0).sin() * decay;
            MotionPose {
                offset: Vec3::new(jolt * 0.08, 0.07 * decay, 0.06 * decay),
                euler: Vec3::new(-0.22 * decay, jolt * 0.12, jolt * 0.44),
                scale_mul: Vec3::ONE,
            }
        }

        // --- Status Ailments & Buffs ---
        MotionId::EmpDisruptedTremor => {
            let hf = (elapsed * 52.0 + seed).sin();
            let lf = (elapsed * 7.5 + seed).cos();
            MotionPose {
                offset: Vec3::new(
                    hf * 0.028,
                    -0.045 + lf * 0.012,
                    (elapsed * 47.0 + seed).cos() * 0.024,
                ),
                euler: Vec3::new(0.2 + lf * 0.06, hf * 0.08, lf * 0.18 + hf * 0.05),
                scale_mul: Vec3::new(1.03, 0.92, 1.03),
            }
        }
        MotionId::BurningFlailPanic => {
            let flail = (elapsed * 16.0 + seed).sin();
            let hop = (elapsed * 12.0 + seed).sin().abs();
            MotionPose {
                offset: Vec3::new(
                    flail * 0.055,
                    hop * 0.06,
                    (elapsed * 14.0 + seed).cos() * 0.035,
                ),
                euler: Vec3::new(-0.14 + flail * 0.14, flail * 0.34, -flail * 0.24),
                scale_mul: Vec3::new(0.95, 1.07 + hop * 0.04, 0.95),
            }
        }
        MotionId::CryoSlowShiver => {
            let shiver = (elapsed * 38.0 + seed).sin();
            MotionPose {
                offset: Vec3::new(shiver * 0.016, -0.035, 0.0),
                euler: Vec3::new(0.16, shiver * 0.05, shiver * 0.07),
                scale_mul: Vec3::new(0.94, 0.93, 0.94),
            }
        }
        MotionId::SuppressedPinnedCrouch => MotionPose {
            offset: Vec3::new((elapsed * 18.0 + seed).sin() * 0.012, -0.11, 0.03),
            euler: Vec3::new(0.32, wave * 0.05, 0.08),
            scale_mul: Vec3::new(1.1, 0.8, 1.1),
        },
        MotionId::ArmorShreddedExposed => MotionPose {
            offset: Vec3::new(wave * 0.024, -0.04, 0.02),
            euler: Vec3::new(0.14, -0.12 + wave * 0.08, -0.15 + cos_w * 0.06),
            scale_mul: Vec3::new(0.96, 0.95, 0.96),
        },
        MotionId::RepairHealingPulse => MotionPose {
            offset: Vec3::new(0.0, 0.05 * bell, 0.0),
            euler: Vec3::new(-0.08 * bell, 0.0, 0.0),
            scale_mul: Vec3::splat(1.0 + 0.08 * bell),
        },
        MotionId::LowHpSmokeStagger => MotionPose {
            offset: Vec3::new(wave * 0.035, -0.05 + cos_w * 0.015, 0.015),
            euler: Vec3::new(0.19 + wave * 0.05, cos_w * 0.06, 0.16 + wave * 0.1),
            scale_mul: Vec3::new(1.04, 0.91, 1.04),
        },
        MotionId::HeroicRallyBuff => MotionPose {
            offset: Vec3::new(0.0, 0.09 * bell, -0.03 * bell),
            euler: Vec3::new(-0.18 * bell, 0.0, 0.0),
            scale_mul: Vec3::new(1.0 + 0.07 * bell, 1.0 + 0.12 * bell, 1.0 + 0.07 * bell),
        },
        MotionId::ChronoHasteOverdrive => {
            let phase = (elapsed * 26.0 + seed).sin();
            MotionPose {
                offset: Vec3::new(phase * 0.015, 0.025 + phase.abs() * 0.02, -0.04),
                euler: Vec3::new(0.16, 0.0, phase * 0.06),
                scale_mul: Vec3::new(0.94, 1.04, 1.08),
            }
        }
        MotionId::ShieldAuraHum => MotionPose {
            offset: Vec3::new(0.0, 0.018 + wave * 0.012, 0.0),
            euler: Vec3::new(-0.03, 0.0, 0.0),
            scale_mul: Vec3::splat(1.015 + wave * 0.015),
        },
        MotionId::LowPowerBrownoutSlump => MotionPose {
            offset: Vec3::new(0.0, -0.07 + wave * 0.008, 0.0),
            euler: Vec3::new(0.24, 0.0, 0.08),
            scale_mul: Vec3::new(1.04, 0.89, 1.04),
        },
        MotionId::VeterancyPromoteSurge => MotionPose {
            offset: Vec3::new(0.0, 0.13 * bell, 0.0),
            euler: Vec3::new(-0.2 * bell, u * TAU * 0.25 * bell, 0.0),
            scale_mul: Vec3::new(1.0 + 0.12 * bell, 1.0 + 0.16 * bell, 1.0 + 0.12 * bell),
        },

        // --- Economy, Construction, Harvest & Interact ---
        MotionId::WorkerHammerBuild => {
            let strike = (u * TAU).sin();
            MotionPose {
                offset: Vec3::new(0.0, -0.02 + strike.abs() * 0.03, -0.04 * strike),
                euler: Vec3::new(0.14 + strike * 0.18, 0.08 * strike, 0.05 * strike),
                scale_mul: Vec3::new(
                    1.0 + 0.04 * strike.max(0.0),
                    1.0 - 0.05 * strike.max(0.0),
                    1.0,
                ),
            }
        }
        MotionId::WorkerWeldSparks => {
            let jitter = (elapsed * 32.0 + seed).sin();
            MotionPose {
                offset: Vec3::new(jitter * 0.015, -0.03, -0.03),
                euler: Vec3::new(0.22, jitter * 0.07, 0.04),
                scale_mul: Vec3::ONE,
            }
        }
        MotionId::HarvesterDrillOre => {
            let drill = (elapsed * 38.0 + seed).sin();
            MotionPose {
                offset: Vec3::new(
                    drill * 0.018,
                    -0.025 + drill.abs() * 0.015,
                    -0.05 + drill * 0.02,
                ),
                euler: Vec3::new(0.18, 0.0, drill * 0.06),
                scale_mul: Vec3::new(1.03, 0.96, 1.04),
            }
        }
        MotionId::HarvesterCrystalExtract => MotionPose {
            offset: Vec3::new(0.0, 0.02 + wave * 0.02, -0.02),
            euler: Vec3::new(0.1, wave * 0.12, cos_w * 0.05),
            scale_mul: Vec3::splat(1.0 + wave * 0.03),
        },
        MotionId::CargoDumpUnload => MotionPose {
            offset: Vec3::new(0.0, 0.03 * bell, 0.05 * bell),
            euler: Vec3::new(-0.32 * bell, 0.0, 0.0),
            scale_mul: Vec3::new(1.05 * bell + (1.0 - bell), 0.95, 1.0),
        },
        MotionId::MineDeployPlant => MotionPose {
            offset: Vec3::new(0.0, -0.09 * bell, 0.0),
            euler: Vec3::new(0.24 * bell, 0.0, 0.0),
            scale_mul: Vec3::new(
                1.08 * bell + (1.0 - bell),
                1.0 - 0.12 * bell,
                1.08 * bell + (1.0 - bell),
            ),
        },
        MotionId::CaptureBeaconChannel => MotionPose {
            offset: Vec3::new(0.0, 0.035 * wave.abs(), 0.0),
            euler: Vec3::new(-0.08, wave * 0.16, 0.0),
            scale_mul: Vec3::new(0.98, 1.04, 0.98),
        },
        MotionId::SabotageChargePlant => MotionPose {
            offset: Vec3::new(0.0, -0.1 * bell, -0.06 * bell),
            euler: Vec3::new(0.32 * bell, -0.16 * bell, 0.0),
            scale_mul: Vec3::new(
                1.06 * bell + (1.0 - bell),
                1.0 - 0.14 * bell,
                1.06 * bell + (1.0 - bell),
            ),
        },
        MotionId::MedicFieldTriage => MotionPose {
            offset: Vec3::new(0.0, -0.04 + wave * 0.02, -0.03),
            euler: Vec3::new(0.16, wave * 0.12, cos_w * 0.05),
            scale_mul: Vec3::ONE,
        },
        MotionId::RadarScanSweep => MotionPose {
            offset: Vec3::new(0.0, 0.015, 0.0),
            euler: Vec3::new(0.0, (u * TAU).sin() * 0.24, 0.0),
            scale_mul: Vec3::ONE,
        },

        // --- Commander, Superunit & Boss Motions ---
        MotionId::SuperunitAwakenRoar => MotionPose {
            offset: Vec3::new(0.0, 0.12 * bell, 0.0),
            euler: Vec3::new(-0.28 * bell, (u * PI * 3.0).sin() * 0.12 * bell, 0.0),
            scale_mul: Vec3::new(1.0 + 0.14 * bell, 1.0 + 0.18 * bell, 1.0 + 0.14 * bell),
        },
        MotionId::SiegeWalkerQuakeStomp => MotionPose {
            offset: Vec3::new(
                0.0,
                0.18 * (1.0 - u) * bell - 0.12 * punch.max(0.0),
                -0.06 * punch,
            ),
            euler: Vec3::new(0.3 * punch, 0.0, 0.1 * punch),
            scale_mul: Vec3::new(
                1.0 + 0.18 * punch.max(0.0),
                1.0 - 0.18 * punch.max(0.0),
                1.0 + 0.18 * punch.max(0.0),
            ),
        },
        MotionId::AirshipBroadsideRoll => MotionPose {
            offset: Vec3::new(0.12 * punch, 0.04 * bell, 0.0),
            euler: Vec3::new(-0.08 * punch, 0.14 * bell, -0.38 * punch),
            scale_mul: Vec3::ONE,
        },
        MotionId::SuperweaponChargeFloat => MotionPose {
            offset: Vec3::new(0.0, 0.12 + wave * 0.045, 0.0),
            euler: Vec3::new(-0.1, wave * 0.14, cos_w * 0.08),
            scale_mul: Vec3::splat(1.06 + wave * 0.05),
        },
        MotionId::BossPhaseOverdrive => {
            let roar = (elapsed * 42.0 + seed).sin() * bell;
            MotionPose {
                offset: Vec3::new(roar * 0.035, 0.14 * bell, 0.0),
                euler: Vec3::new(-0.26 * bell, roar * 0.14, roar * 0.12),
                scale_mul: Vec3::splat(1.0 + 0.18 * bell),
            }
        }
        MotionId::TitanCannonBrace => MotionPose {
            offset: Vec3::new(0.0, -0.09 * u, 0.08 * u),
            euler: Vec3::new(-0.18 * u, 0.0, 0.0),
            scale_mul: Vec3::new(1.0 + 0.13 * u, 1.0 - 0.12 * u, 1.0 + 0.1 * u),
        },
        MotionId::DreadnoughtBarrageRecoil => {
            let ripple = (u * PI * 5.0).sin() * decay;
            MotionPose {
                offset: Vec3::new(ripple * 0.04, -0.06 * punch, 0.22 * punch),
                euler: Vec3::new(-0.28 * punch, ripple * 0.1, -ripple * 0.12),
                scale_mul: Vec3::new(1.0 + 0.12 * punch, 1.0 - 0.1 * punch, 1.0 - 0.14 * punch),
            }
        }
        MotionId::CommanderTacticalPoint => MotionPose {
            offset: Vec3::new(0.0, 0.03 * bell, -0.06 * bell),
            euler: Vec3::new(0.14 * bell, -0.18 * bell, 0.0),
            scale_mul: Vec3::new(0.98, 1.05 * bell + (1.0 - bell), 1.02),
        },
        MotionId::CommanderOrbitalCall => MotionPose {
            offset: Vec3::new(0.0, 0.08 * bell, 0.02 * bell),
            euler: Vec3::new(-0.26 * bell, 0.12 * bell, 0.0),
            scale_mul: Vec3::new(0.97, 1.1 * bell + (1.0 - bell), 0.97),
        },
        MotionId::BossCoreMeltdown => {
            let quake = (elapsed * 60.0 + seed).sin();
            MotionPose {
                offset: Vec3::new(
                    quake * 0.06 * u,
                    0.08 * u,
                    (elapsed * 49.0).cos() * 0.06 * u,
                ),
                euler: Vec3::new(-0.2 * u, quake * 0.22 * u, quake * 0.28 * u),
                scale_mul: Vec3::splat(1.0 + 0.22 * (u * PI).sin()),
            }
        }

        // --- Co-op, Squad Formation, Rally & Social ---
        MotionId::SquadHoldLineBrace => MotionPose {
            offset: Vec3::new(0.0, -0.055 + wave * 0.008, 0.02),
            euler: Vec3::new(0.14, 0.0, 0.0),
            scale_mul: Vec3::new(1.06, 0.91, 1.06),
        },
        MotionId::SquadAdvanceWave => MotionPose {
            offset: Vec3::new(0.0, 0.04 * bell, -0.07 * bell),
            euler: Vec3::new(0.18 * bell, 0.14 * bell, -0.08 * bell),
            scale_mul: Vec3::new(0.98, 1.05 * bell + (1.0 - bell), 1.03),
        },
        MotionId::SquadFormationAlign => MotionPose {
            offset: Vec3::new((u * PI * 2.0).sin() * 0.05, 0.015 * bell, 0.0),
            euler: Vec3::new(0.04 * bell, (u * PI * 2.0).sin() * 0.12, 0.0),
            scale_mul: Vec3::ONE,
        },
        MotionId::RallyFlagPlant => MotionPose {
            offset: Vec3::new(
                0.0,
                0.08 * (1.0 - u) * bell - 0.06 * punch.max(0.0),
                -0.05 * bell,
            ),
            euler: Vec3::new(0.2 * punch, 0.0, 0.0),
            scale_mul: Vec3::new(
                1.0 + 0.08 * punch.max(0.0),
                1.0 - 0.1 * punch.max(0.0),
                1.0 + 0.08 * punch.max(0.0),
            ),
        },
        MotionId::TacticalPingSignal => MotionPose {
            offset: Vec3::new(0.0, 0.06 * bell, 0.0),
            euler: Vec3::new(-0.14 * bell, (u * PI * 2.0).sin() * 0.2, 0.0),
            scale_mul: Vec3::new(0.97, 1.08 * bell + (1.0 - bell), 0.97),
        },
        MotionId::OfficerSalute => MotionPose {
            offset: Vec3::new(0.0, 0.035 * bell, 0.0),
            euler: Vec3::new(-0.12 * bell, 0.0, -0.06 * bell),
            scale_mul: Vec3::new(0.96, 1.07 * bell + (1.0 - bell), 0.96),
        },
        MotionId::SquadVictoryCheer => {
            let hop = (u * PI).sin().abs();
            MotionPose {
                offset: Vec3::new(0.0, hop * 0.14, 0.0),
                euler: Vec3::new(-0.16 * hop, (u * TAU).sin() * 0.18, (u * TAU).cos() * 0.1),
                scale_mul: Vec3::new(1.0 - 0.06 * hop, 1.0 + 0.12 * hop, 1.0 - 0.06 * hop),
            }
        }
        MotionId::CoopAssistAcknowledge => MotionPose {
            offset: Vec3::new(0.0, 0.03 * bell, -0.03 * bell),
            euler: Vec3::new(0.16 * (u * PI * 2.0).sin(), 0.0, 0.0),
            scale_mul: Vec3::ONE,
        },
        MotionId::FlankSignalSweep => MotionPose {
            offset: Vec3::new(0.05 * bell, 0.02 * bell, -0.04 * bell),
            euler: Vec3::new(0.1 * bell, 0.3 * bell, -0.14 * bell),
            scale_mul: Vec3::ONE,
        },
        MotionId::RetreatRegroupWave => MotionPose {
            offset: Vec3::new(0.0, 0.03 * bell, 0.06 * bell),
            euler: Vec3::new(-0.14 * bell, (u * PI * 3.0).sin() * 0.24 * bell, 0.0),
            scale_mul: Vec3::ONE,
        },

        // --- Victory, Defeat & Death Sequences ---
        MotionId::VictoryFistPump => {
            let hop = (u * PI).sin().abs();
            MotionPose {
                offset: Vec3::new(0.0, hop * 0.16, 0.0),
                euler: Vec3::new(-0.2 * hop, 0.0, (u * TAU).sin() * 0.12),
                scale_mul: Vec3::new(1.0 - 0.08 * hop, 1.0 + 0.14 * hop, 1.0 - 0.08 * hop),
            }
        }
        MotionId::MatchDefeatSlump => MotionPose {
            offset: Vec3::new(0.0, -0.1 + wave * 0.008, 0.02),
            euler: Vec3::new(0.36, 0.0, 0.12),
            scale_mul: Vec3::new(1.05, 0.86, 1.05),
        },
        MotionId::InfantryFallBack => MotionPose {
            offset: Vec3::new(0.0, -0.18 * u, 0.22 * u),
            euler: Vec3::new(-0.95 * u, 0.18 * u, 0.24 * u),
            scale_mul: Vec3::new(1.08 * u + (1.0 - u), 1.0 - 0.16 * u, 1.08 * u + (1.0 - u)),
        },
        MotionId::VehicleCookoffBuck => {
            let shudder = (elapsed * 52.0 + seed).sin() * decay;
            MotionPose {
                offset: Vec3::new(shudder * 0.05, 0.14 * bell, 0.08 * u),
                euler: Vec3::new(-0.32 * bell, shudder * 0.2, 0.34 * u),
                scale_mul: Vec3::new(1.0 + 0.16 * bell, 1.0 - 0.14 * u, 1.0 + 0.16 * bell),
            }
        }
        MotionId::MechCollapseKneel => MotionPose {
            offset: Vec3::new(0.0, -0.22 * u, 0.08 * u),
            euler: Vec3::new(0.58 * u, -0.12 * u, 0.26 * u),
            scale_mul: Vec3::new(1.1 * u + (1.0 - u), 1.0 - 0.2 * u, 1.1 * u + (1.0 - u)),
        },
        MotionId::AirSpiralCrash => MotionPose {
            offset: Vec3::new((u * TAU).sin() * 0.18 * u, -0.24 * u, -0.14 * u),
            euler: Vec3::new(0.65 * u, u * 0.8, 0.95 * u),
            scale_mul: Vec3::ONE,
        },
    };

    pose.sanitized()
}

/// Maps a local-space hit direction (where the attack came *from* in the
/// defender's local frame, with forward = -Z) to a 4-way directional flinch.
pub(crate) fn directional_flinch_for_local_hit(local_from_dir: Vec3) -> MotionId {
    if local_from_dir.x.abs() > local_from_dir.z.abs() {
        if local_from_dir.x < 0.0 {
            MotionId::FlinchLeft
        } else {
            MotionId::FlinchRight
        }
    } else if local_from_dir.z <= 0.0 {
        // Hit arrived from front (-Z) -> unit snaps backward.
        MotionId::FlinchFront
    } else {
        // Hit arrived from behind (+Z) -> unit lurches forward.
        MotionId::FlinchBack
    }
}

/// Presentation-only per-unit procedural motion state with critically-damped
/// pose blending and 62.5Hz local impact micro-tremor.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct MotionState {
    pub(crate) active: MotionId,
    pub(crate) elapsed: f32,
    pub(crate) seed: f32,
    pub(crate) current_pose: MotionPose,
    pub(crate) offset_velocity: Vec3,
    pub(crate) euler_velocity: Vec3,
    pub(crate) scale_velocity: Vec3,
    // Impact micro-tremor (local hitstop feel without stalling RTS clock)
    pub(crate) tremor_remaining: f32,
    pub(crate) tremor_duration: f32,
    pub(crate) tremor_intensity: f32,
    pub(crate) tremor_local_dir: Vec3,
    // Presentation-only edge detectors (never read by simulation)
    pub(crate) last_health: f32,
    pub(crate) was_moving: bool,
    pub(crate) was_emp: bool,
    pub(crate) was_deployed: bool,
    pub(crate) last_veterancy_rank: u8,
    pub(crate) last_yaw: f32,
    pub(crate) last_shot_cooldown_left: f32,
}

impl MotionState {
    pub(crate) fn new(seed: f32, initial_health: f32, initial_yaw: f32) -> Self {
        Self {
            active: MotionId::IdleReady,
            elapsed: 0.0,
            seed,
            current_pose: MotionPose::IDENTITY,
            offset_velocity: Vec3::ZERO,
            euler_velocity: Vec3::ZERO,
            scale_velocity: Vec3::ZERO,
            tremor_remaining: 0.0,
            tremor_duration: IMPACT_TREMOR_DURATION_SEC,
            tremor_intensity: 0.0,
            tremor_local_dir: Vec3::NEG_Z,
            last_health: initial_health,
            was_moving: false,
            was_emp: false,
            was_deployed: false,
            last_veterancy_rank: 0,
            last_yaw: initial_yaw,
            last_shot_cooldown_left: 0.0,
        }
    }

    /// Requests a new motion respecting priority and completion rules.
    pub(crate) fn request(&mut self, next: MotionId, force_restart: bool) {
        if self.active == next && !force_restart {
            return;
        }
        let cur_spec = self.active.spec();
        let next_spec = next.spec();
        let cur_finished = !cur_spec.looping && self.elapsed >= cur_spec.duration;
        if force_restart
            || cur_finished
            || next_spec.priority >= cur_spec.priority
            || cur_spec.looping
        {
            self.active = next;
            self.elapsed = 0.0;
        }
    }

    /// Triggers a 62.5Hz zero-drift impact micro-tremor + directional flinch.
    pub(crate) fn trigger_hit_impact(
        &mut self,
        local_from_dir: Vec3,
        severity: f32,
        reaction_motion: MotionId,
    ) {
        let clamped = severity.clamp(0.25, 1.5);
        self.tremor_duration = IMPACT_TREMOR_DURATION_SEC + 0.03 * (clamped - 0.5).max(0.0);
        self.tremor_remaining = self.tremor_duration;
        self.tremor_intensity = clamped;
        self.tremor_local_dir = local_from_dir.normalize_or(Vec3::NEG_Z);
        self.request(reaction_motion, true);
    }

    /// Advances the motion clock, tremor envelope, and critically-damped pose.
    pub(crate) fn step(&mut self, dt: f32, global_clock: f32) -> MotionPose {
        let dt = dt.clamp(0.0, 0.1);
        let spec = self.active.spec();
        self.elapsed += dt;
        let u = if spec.looping {
            (self.elapsed / spec.duration).fract()
        } else {
            (self.elapsed / spec.duration).min(1.0)
        };
        let mut target = evaluate_motion_pose(self.active, u, global_clock, self.seed);

        if self.tremor_remaining > 0.0 {
            self.tremor_remaining = (self.tremor_remaining - dt).max(0.0);
            let tremor = evaluate_impact_tremor(
                self.tremor_remaining,
                self.tremor_duration,
                self.tremor_intensity,
                self.tremor_local_dir,
                global_clock,
            );
            target.offset += tremor.offset;
            target.euler += tremor.euler;
            target.scale_mul *= tremor.scale_mul;
            target = target.sanitized();
        }

        // Critically-damped spring blend (snappier on high-priority hits/recoil).
        let omega = match spec.category {
            MotionCategory::HitReaction | MotionCategory::AttackRecoil => 28.0,
            MotionCategory::ChargeWindup | MotionCategory::EvasiveRecovery => 22.0,
            _ => 16.0,
        };
        self.current_pose = MotionPose {
            offset: critically_damped_vec3(
                self.current_pose.offset,
                target.offset,
                &mut self.offset_velocity,
                omega,
                dt,
            ),
            euler: critically_damped_vec3(
                self.current_pose.euler,
                target.euler,
                &mut self.euler_velocity,
                omega,
                dt,
            ),
            scale_mul: critically_damped_vec3(
                self.current_pose.scale_mul,
                target.scale_mul,
                &mut self.scale_velocity,
                omega,
                dt,
            ),
        }
        .sanitized();

        self.current_pose
    }
}

/// Tags a direct visual child of a `Unit` so `apply_unit_motion_poses` can
/// pose it cleanly from its saved rest transform with zero drift and without
/// ever writing the unit root's `Transform`.
#[derive(Component, Clone, Copy)]
pub(crate) struct MotionRigPart {
    pub(crate) root: Entity,
    pub(crate) rest_translation: Vec3,
    pub(crate) rest_rotation: Quat,
    pub(crate) rest_scale: Vec3,
}

fn is_walker_unit_id(id: &str) -> bool {
    matches!(
        id,
        "AntiAirWalker" | "HeavySiegeWalker" | "RailArtilleryWalker" | "RocketTrooperRobot"
    )
}

fn attack_recoil_motion_for_unit(id: &str, weapon: &Weapon, deployed: bool) -> MotionId {
    if deployed {
        return MotionId::SiegeShellBlastRecoil;
    }
    match id {
        "RailgunTank" | "RailArtilleryWalker" => MotionId::RailgunHyperKick,
        "RailSniperTeam" | "SniperScout" => MotionId::SniperSnapRecoil,
        "FlameAssaultBuggy" => MotionId::FlameSweepArc,
        "CryoSprayer" => MotionId::CryoStreamSweep,
        "TeslaCrawlerMk2" | "ShockTrooper" => MotionId::TeslaArcDischarge,
        "LanceBeamTank" | "PulseRifleCommando" => MotionId::BeamChannelSustain,
        "MortarTeam" => MotionId::MortarLaunchThump,
        "GrenadierTrooper" => MotionId::GrenadeLobThrow,
        "SiegeDrillTank" => MotionId::DrillBoreThrust,
        "HeavySiegeWalker" => MotionId::MechStompSlam,
        "HeavyBombardmentAirship" | "SiegeAirship" => MotionId::AirshipBroadsideRoll,
        "BomberVTOL" => MotionId::BomberPayloadDrop,
        "FlakHoverTank" | "FlakRocketTeam" | "FlakRocketTeamMk2" | "AntiAirWalker" => {
            MotionId::FlakSkywardBurst
        }
        "RocketInfantry"
        | "RocketTrooperRobot"
        | "LongbowMissileCrawler"
        | "ModularMissileCarrier"
        | "RocketGunship" => MotionId::RocketSalvoLaunch,
        "HeavyMachinegunTrooper" => MotionId::HeavyMachinegunSpray,
        "Tank" | "HammerSiegeTank" | "SiegeArtilleryVehicle" => MotionId::HeavyCannonKickback,
        "MirageScoutTank" | "ScoutRover" | "InterceptorVTOL" | "Helicopter" => {
            MotionId::TwinAutocannonRecoil
        }
        _ if weapon.splash_radius > 0.0 => MotionId::HeavyCannonKickback,
        _ if weapon.damage >= 18.0 => MotionId::HeavyCannonKickback,
        _ => MotionId::RifleBurstFire,
    }
}

fn charge_windup_motion_for_unit(id: &str, deployed: bool) -> MotionId {
    if deployed {
        return MotionId::ArtilleryBrace;
    }
    match id {
        "RailgunTank" | "RailArtilleryWalker" => MotionId::RailgunCharge,
        "RailSniperTeam" | "SniperScout" => MotionId::SniperBreathHold,
        "TeslaCrawlerMk2" | "ShockTrooper" => MotionId::TeslaCoilSpool,
        "LanceBeamTank" | "PulseRifleCommando" => MotionId::BeamFocusSpool,
        "CryoSprayer" => MotionId::CryoTankPressurize,
        "FlameAssaultBuggy" => MotionId::FlamerPilotIgnite,
        "MortarTeam" => MotionId::MortarTubeAim,
        "SiegeDrillTank" => MotionId::SiegeDrillRevUp,
        "RocketInfantry"
        | "RocketTrooperRobot"
        | "LongbowMissileCrawler"
        | "ModularMissileCarrier"
        | "RocketGunship" => MotionId::MissileLockOn,
        "HeavySiegeWalker" => MotionId::TitanCannonBrace,
        "TacticalOfficer" => MotionId::OrbitalDesignatorLock,
        _ => MotionId::PlasmaOvercharge,
    }
}

/// Marks a `Unit` whose direct recentered visual children have already been
/// scanned for `MotionRigPart`, avoiding per-frame child list walks.
#[derive(Component)]
pub(crate) struct MotionRigBound;

/// Ensures every live `Unit` has a presentation-only `MotionState` and tags
/// its direct visual children (after `ModelRecentered` settles) with
/// `MotionRigPart`.
#[allow(clippy::type_complexity)]
pub(crate) fn ensure_unit_motion_rigs(
    mut commands: Commands,
    units_without_state: Query<(Entity, &Health, &Transform), (With<Unit>, Without<MotionState>)>,
    recentered_units: Query<
        (Entity, &Children),
        (With<Unit>, With<ModelRecentered>, Without<MotionRigBound>),
    >,
    candidate_parts: Query<
        (&Transform, Option<&Name>),
        (
            Without<MotionRigPart>,
            Without<AirframePart>,
            Without<TurretNode>,
            Without<TankSpinner>,
            Without<ContextualFactionMarker>,
            Without<DeployOutrigger>,
            Or<(With<WorldAssetRoot>, With<Mesh3d>)>,
        ),
    >,
) {
    for (entity, health, transform) in &units_without_state {
        let (yaw, _, _) = transform.rotation.to_euler(EulerRot::YXZ);
        let seed = (entity.to_bits() % 131) as f32 * 0.29;
        commands
            .entity(entity)
            .try_insert(MotionState::new(seed, health.current, yaw));
    }
    for (root, children) in &recentered_units {
        for child in children.iter() {
            let Ok((transform, name)) = candidate_parts.get(child) else {
                continue;
            };
            // Never claim a turret child before `tag_turret_nodes` tags it.
            if name.is_some_and(|n| n.as_str() == "turret") {
                continue;
            }
            commands.entity(child).try_insert(MotionRigPart {
                root,
                rest_translation: transform.translation,
                rest_rotation: transform.rotation,
                rest_scale: transform.scale,
            });
        }
        commands.entity(root).try_insert(MotionRigBound);
    }
}

type UnitMotionCore<'a> = (
    &'a Unit,
    &'a Team,
    &'a Transform,
    &'a Health,
    &'a MovementDomain,
    Option<&'a VisibilityState>,
    &'a mut MotionState,
);

type UnitMotionOrders<'a> = (
    Option<&'a Weapon>,
    Option<&'a AttackOrder>,
    Option<&'a MoveOrder>,
    Option<&'a HarvestOrder>,
    Option<&'a HoldPosition>,
    Option<&'a Veterancy>,
);

type UnitMotionStatuses<'a> = (
    Has<ConstructionWorkPulse>,
    Has<EmpDisabled>,
    Has<DeployedSiegeMode>,
    Has<ChronoRelay>,
    Has<SupportShield>,
    Has<PassiveSupportShield>,
    Option<&'a FactionShield>,
);

type UnitMotionQuery<'a> = (
    UnitMotionCore<'a>,
    UnitMotionOrders<'a>,
    UnitMotionStatuses<'a>,
);

/// Reads live unit combat/movement/status state (never writes simulation state)
/// and updates each unit's `MotionState` + triggers skid dust & siege shock-rings.
#[allow(clippy::too_many_arguments)]
pub(crate) fn update_unit_motions(
    mut commands: Commands,
    time: Res<Time>,
    match_state: Option<Res<MatchState>>,
    visible_player: Option<Res<VisiblePlayer>>,
    recent_pulses: Query<&ShotPulse>,
    mut units: Query<UnitMotionQuery>,
) {
    let dt = time.delta_secs();
    let elapsed = time.elapsed_secs();
    let inv_dt = 1.0 / dt.max(1e-4);

    for (
        (unit, team, transform, health, domain, visibility, mut motion),
        (weapon, attack_order, move_order, harvest_order, hold_position, veterancy),
        (
            building,
            emp_disabled,
            deployed,
            chrono_haste,
            support_shield,
            passive_shield,
            faction_shield,
        ),
    ) in &mut units
    {
        let visible = visibility.is_none_or(|v| v.visible);
        let infantry = is_infantry_unit(unit);
        let walker = is_walker_unit_id(unit.id);
        let airborne = *domain == MovementDomain::Air;
        let moving = move_order.is_some();
        let shielded =
            support_shield || passive_shield || faction_shield.is_some_and(|s| s.current > 0.0);
        let hp_ratio = health.ratio();

        let (yaw, _, _) = transform.rotation.to_euler(EulerRot::YXZ);
        let yaw_rate = wrap_angle(yaw - motion.last_yaw) * inv_dt;
        motion.last_yaw = yaw;

        // 1. Match-end victory / defeat celebration or slump.
        if let Some(state) = match_state.as_deref()
            && !state.is_running()
        {
            let human_team = visible_player
                .as_deref()
                .map_or(Team::Player(0), |vp| vp.team);
            let won = match state.phase {
                MatchPhase::HumanVictory => *team == human_team,
                MatchPhase::HumanDefeat => *team != human_team,
                _ => false,
            };
            motion.request(
                if won {
                    if infantry {
                        MotionId::VictoryFistPump
                    } else {
                        MotionId::SquadVictoryCheer
                    }
                } else {
                    MotionId::MatchDefeatSlump
                },
                false,
            );
            motion.step(dt, elapsed);
            continue;
        }

        // 2. Damage / hit detection -> directional flinch + 62.5Hz micro-tremor.
        let health_delta = health.current - motion.last_health;
        motion.last_health = health.current;
        if health_delta < -0.05 && health.current > 0.0 {
            let dmg_frac = (-health_delta / health.max.max(1.0)).clamp(0.0, 1.0);
            // Infer hit direction from the closest active tracer landing near this unit.
            let mut local_hit_dir = Vec3::NEG_Z;
            let mut best_dist_sq = 2.5_f32 * 2.5;
            for pulse in &recent_pulses {
                let d_sq = pulse.to.distance_squared(transform.translation);
                if d_sq < best_dist_sq {
                    best_dist_sq = d_sq;
                    let world_from = (pulse.from - pulse.to).normalize_or(Vec3::NEG_Z);
                    local_hit_dir = transform.rotation.inverse() * world_from;
                }
            }
            let reaction = if airborne {
                MotionId::AirTurbulenceJolt
            } else if shielded && dmg_frac < 0.16 {
                MotionId::ShieldBlockBrace
            } else if dmg_frac >= 0.22 {
                MotionId::CritStaggerReel
            } else if !infantry && !walker && dmg_frac < 0.14 {
                MotionId::HeavyArmorRattle
            } else {
                directional_flinch_for_local_hit(local_hit_dir)
            };
            let severity = (0.55 + dmg_frac * 2.4).clamp(0.55, 1.45);
            motion.trigger_hit_impact(local_hit_dir, severity, reaction);
        } else if health_delta > 0.2 {
            motion.request(MotionId::RepairHealingPulse, false);
        }

        // 3. Veterancy promotion flourish.
        if let Some(vet) = veterancy {
            if vet.rank > motion.last_veterancy_rank {
                motion.request(MotionId::VeterancyPromoteSurge, true);
            }
            motion.last_veterancy_rank = vet.rank;
        }

        // 4. EMP disabled entry wobble -> sustained electrical tremor.
        if emp_disabled {
            if !motion.was_emp {
                motion.request(MotionId::EmpStunWobble, true);
            } else if motion.active != MotionId::EmpStunWobble {
                motion.request(MotionId::EmpDisruptedTremor, false);
            }
        }
        motion.was_emp = emp_disabled;

        // 5. Siege deploy / undeploy transitions + ground shock-ring on anchor!
        if deployed && !motion.was_deployed {
            motion.request(MotionId::SiegeDeployAnchor, true);
            if visible {
                crate::combat_feel::spawn_additive_shock_ring(
                    &mut commands,
                    transform.translation,
                    0.28,
                    1.35,
                    0.28,
                    LinearRgba::new(1.35, 0.95, 0.35, 1.0),
                );
            }
        } else if !deployed && motion.was_deployed {
            motion.request(MotionId::SiegeUndeployPack, true);
        }
        motion.was_deployed = deployed;

        // 6. Weapon firing & charge windup detection.
        if let Some(weapon) = weapon {
            let just_fired = weapon.cooldown_left > motion.last_shot_cooldown_left + 0.05;
            motion.last_shot_cooldown_left = weapon.cooldown_left;
            if just_fired && !emp_disabled {
                let recoil = attack_recoil_motion_for_unit(unit.id, weapon, deployed);
                motion.request(recoil, true);
            } else if !emp_disabled
                && attack_order.is_some()
                && weapon.cooldown >= 1.15
                && weapon.cooldown_left > 0.0
                && weapon.cooldown_left < (weapon.cooldown * 0.28).min(0.45)
            {
                let charge = charge_windup_motion_for_unit(unit.id, deployed);
                motion.request(charge, false);
            }
        }

        // 7. Abrupt stop skid + tread/boot dust puff (visible units only)!
        if motion.was_moving && !moving && !airborne && !emp_disabled {
            let skid = if infantry {
                MotionId::SprintSkidStop
            } else {
                MotionId::TankBrakeDip
            };
            motion.request(skid, false);
            if visible {
                crate::combat_feel::spawn_tread_dust_puff(
                    &mut commands,
                    transform.translation,
                    *transform.forward(),
                    if infantry { 0.55 } else { 1.05 },
                );
            }
        }
        motion.was_moving = moving;

        // 8. Ongoing activity / status / locomotion selection.
        if !emp_disabled {
            let desired = if building {
                MotionId::WorkerHammerBuild
            } else if harvest_order.is_some_and(|h| h.state == HarvestState::Collecting) {
                MotionId::HarvesterDrillOre
            } else if moving {
                if airborne {
                    if attack_order.is_some() {
                        MotionId::AirDiveStrafe
                    } else if yaw_rate > 0.55 {
                        MotionId::HoverBankLeft
                    } else if yaw_rate < -0.55 {
                        MotionId::HoverBankRight
                    } else {
                        MotionId::AirCruiseHover
                    }
                } else if hp_ratio < 0.35 && infantry {
                    MotionId::LowHpLimp
                } else if chrono_haste {
                    MotionId::ChronoHasteOverdrive
                } else if walker {
                    MotionId::MechHeavyStride
                } else if infantry {
                    if unit.speed >= 3.2 {
                        MotionId::InfantrySprint
                    } else {
                        MotionId::InfantryMarch
                    }
                } else {
                    MotionId::TankTreadRoll
                }
            } else if hp_ratio < 0.25 {
                if infantry {
                    MotionId::BurningFlailPanic
                } else {
                    MotionId::LowHpSmokeStagger
                }
            } else if hold_position.is_some_and(|h| h.enabled) {
                MotionId::SquadHoldLineBrace
            } else if airborne {
                MotionId::AirCruiseHover
            } else if shielded {
                MotionId::ShieldAuraHum
            } else if unit.id == "TacticalOfficer" {
                MotionId::IdleGuard
            } else {
                MotionId::IdleReady
            };
            motion.request(desired, false);
        }

        motion.step(dt, elapsed);
    }
}

/// Applies each unit's evaluated `MotionPose` to its child visual rigs
/// (`MotionRigPart`, plus composes onto `AirframePart` and direct `TurretNode`
/// children) — NEVER writing the root `Unit` entity's `Transform`!
#[allow(clippy::type_complexity)]
pub(crate) fn apply_unit_motion_poses(
    motions: Query<&MotionState, With<Unit>>,
    mut rig_parts: Query<
        (&MotionRigPart, &mut Transform),
        (
            Without<AirframePart>,
            Without<TurretNode>,
            Without<TankSpinner>,
        ),
    >,
    mut air_parts: Query<
        (&AirframePart, &mut Transform),
        (
            Without<MotionRigPart>,
            Without<TurretNode>,
            Without<TankSpinner>,
        ),
    >,
    mut direct_turrets: Query<
        (&TurretNode, &ChildOf, &mut Transform),
        (
            Without<MotionRigPart>,
            Without<AirframePart>,
            Without<TankSpinner>,
        ),
    >,
    mut direct_spinners: Query<
        (&ChildOf, &mut Transform),
        (
            With<TankSpinner>,
            Without<MotionRigPart>,
            Without<AirframePart>,
            Without<TurretNode>,
        ),
    >,
) {
    for (part, mut transform) in &mut rig_parts {
        let Ok(motion) = motions.get(part.root) else {
            continue;
        };
        let pose = motion.current_pose;
        let rot = pose.rotation();
        transform.translation = rot * part.rest_translation + pose.offset;
        transform.rotation = rot * part.rest_rotation;
        transform.scale = part.rest_scale * pose.scale_mul;
    }

    for (part, mut transform) in &mut air_parts {
        let Ok(motion) = motions.get(part.root) else {
            continue;
        };
        let pose = motion.current_pose;
        let rot = pose.rotation();
        transform.translation += pose.offset;
        transform.rotation = rot * transform.rotation;
        transform.scale = pose.scale_mul;
    }

    // For procedural tanks, turret & spinner are direct children of the unit root,
    // so compose the hull's MotionPose onto their freshly-animated transforms.
    for (turret, child_of, mut transform) in &mut direct_turrets {
        if child_of.0 != turret.root {
            continue;
        }
        let Ok(motion) = motions.get(turret.root) else {
            continue;
        };
        let pose = motion.current_pose;
        let rot = pose.rotation();
        transform.translation = rot * transform.translation + pose.offset;
        transform.rotation = rot * transform.rotation;
        transform.scale = pose.scale_mul;
    }

    for (child_of, mut transform) in &mut direct_spinners {
        let Ok(motion) = motions.get(child_of.0) else {
            continue;
        };
        let pose = motion.current_pose;
        transform.scale = pose.scale_mul;
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn catalog_has_at_least_100_unique_valid_motions() {
        assert!(
            MotionId::ALL.len() >= 100,
            "expected >= 100 motions, got {}",
            MotionId::ALL.len()
        );
        let mut seen_ids = HashSet::new();
        let mut seen_zh = HashSet::new();
        let mut seen_en = HashSet::new();
        let mut categories = HashSet::new();

        for id in MotionId::ALL {
            let spec = id.spec();
            assert_eq!(spec.id, id);
            assert!(seen_ids.insert(id), "duplicate MotionId {id:?}");
            assert!(!spec.name_zh.trim().is_empty(), "empty zh name for {id:?}");
            assert!(!spec.name_en.trim().is_empty(), "empty en name for {id:?}");
            assert!(
                seen_zh.insert(spec.name_zh),
                "duplicate zh name {} for {id:?}",
                spec.name_zh
            );
            assert!(
                seen_en.insert(spec.name_en),
                "duplicate en name {} for {id:?}",
                spec.name_en
            );
            assert!(spec.duration > 0.0 && spec.duration.is_finite());
            assert!(spec.priority <= 100);
            categories.insert(spec.category);

            // Every motion must evaluate to finite, bounded poses across its cycle.
            for step in 0..=8 {
                let u = step as f32 / 8.0;
                let pose = evaluate_motion_pose(id, u, u * spec.duration, 0.7);
                assert!(pose.offset.is_finite(), "{id:?} non-finite offset");
                assert!(pose.euler.is_finite(), "{id:?} non-finite euler");
                assert!(pose.scale_mul.is_finite(), "{id:?} non-finite scale");
                assert!(
                    pose.scale_mul.min_element() > 0.5 && pose.scale_mul.max_element() < 1.5,
                    "{id:?} unsafe scale {:?}",
                    pose.scale_mul
                );
            }
        }
        // All 10 RTS categories must be represented.
        assert_eq!(categories.len(), 10);
    }

    #[test]
    fn key_combat_and_status_motions_produce_distinct_poses() {
        let samples = [
            (
                "railgun_charge",
                evaluate_motion_pose(MotionId::RailgunCharge, 0.75, 0.36, 0.0),
            ),
            (
                "emp_tremor",
                evaluate_motion_pose(MotionId::EmpDisruptedTremor, 0.4, 0.24, 0.0),
            ),
            (
                "burning_flail",
                evaluate_motion_pose(MotionId::BurningFlailPanic, 0.35, 0.17, 0.0),
            ),
            (
                "crit_stagger",
                evaluate_motion_pose(MotionId::CritStaggerReel, 0.25, 0.1, 0.0),
            ),
            (
                "siege_recoil",
                evaluate_motion_pose(MotionId::SiegeShellBlastRecoil, 0.3, 0.14, 0.0),
            ),
            (
                "cryo_shiver",
                evaluate_motion_pose(MotionId::CryoSlowShiver, 0.5, 0.28, 0.0),
            ),
            (
                "squad_cheer",
                evaluate_motion_pose(MotionId::SquadVictoryCheer, 0.5, 0.45, 0.0),
            ),
        ];

        for i in 0..samples.len() {
            for j in (i + 1)..samples.len() {
                let (name_a, a) = samples[i];
                let (name_b, b) = samples[j];
                let diff = (a.offset - b.offset).length()
                    + (a.euler - b.euler).length()
                    + (a.scale_mul - b.scale_mul).length();
                assert!(
                    diff > 0.08,
                    "{name_a} and {name_b} poses are too similar (diff={diff:.4})"
                );
            }
        }
    }

    #[test]
    fn directional_hitstun_distinguishes_all_four_quadrants() {
        assert_eq!(
            directional_flinch_for_local_hit(Vec3::NEG_Z),
            MotionId::FlinchFront
        );
        assert_eq!(
            directional_flinch_for_local_hit(Vec3::Z),
            MotionId::FlinchBack
        );
        assert_eq!(
            directional_flinch_for_local_hit(Vec3::NEG_X),
            MotionId::FlinchLeft
        );
        assert_eq!(
            directional_flinch_for_local_hit(Vec3::X),
            MotionId::FlinchRight
        );

        let front = evaluate_motion_pose(MotionId::FlinchFront, 0.15, 0.05, 0.0);
        let back = evaluate_motion_pose(MotionId::FlinchBack, 0.15, 0.05, 0.0);
        let left = evaluate_motion_pose(MotionId::FlinchLeft, 0.15, 0.05, 0.0);
        let right = evaluate_motion_pose(MotionId::FlinchRight, 0.15, 0.05, 0.0);

        assert!(front.offset.z > 0.0 && back.offset.z < 0.0);
        assert!(front.euler.x < 0.0 && back.euler.x > 0.0);
        assert!(left.offset.x > 0.0 && right.offset.x < 0.0);
        assert!(left.euler.z < 0.0 && right.euler.z > 0.0);
    }

    #[test]
    fn anticipation_curve_pulls_back_snaps_and_settles() {
        let anti = anticipation_snap_recoil(0.09);
        let peak = anticipation_snap_recoil(0.34);
        let end = anticipation_snap_recoil(1.0);
        assert!(anti < -0.1, "expected negative anticipation, got {anti}");
        assert!(peak > 0.85, "expected explosive peak near 1.0, got {peak}");
        assert!(end.abs() < 0.05, "expected settled end, got {end}");
    }

    #[test]
    fn impact_tremor_vibrates_at_high_frequency_and_decays_to_exact_zero() {
        let active = evaluate_impact_tremor(0.09, 0.11, 1.0, Vec3::NEG_Z, 0.004);
        assert!(active.offset.length() > 0.005);
        assert!(active.scale_mul.y < 0.99);
        assert!(active.scale_mul.x > 1.0);

        let settled = evaluate_impact_tremor(0.0, 0.11, 1.0, Vec3::NEG_Z, 0.5);
        assert_eq!(settled, MotionPose::IDENTITY);
    }

    #[test]
    fn motion_state_blends_smoothly_and_respects_priority() {
        let mut state = MotionState::new(0.2, 100.0, 0.0);
        state.trigger_hit_impact(Vec3::NEG_Z, 1.1, MotionId::CritStaggerReel);
        assert_eq!(state.active, MotionId::CritStaggerReel);
        // Low-priority idle cannot interrupt an active stagger mid-flight.
        state.request(MotionId::IdleReady, false);
        assert_eq!(state.active, MotionId::CritStaggerReel);

        let pose_mid = state.step(0.03, 0.03);
        assert!(pose_mid.offset.length() > 0.001);

        // Once the one-shot finishes, idle takes over cleanly.
        for i in 1..=20 {
            state.step(0.03, 0.03 + i as f32 * 0.03);
        }
        state.request(MotionId::IdleReady, false);
        assert_eq!(state.active, MotionId::IdleReady);
    }
}
