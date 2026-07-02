# 第三梯队设计:地形障碍 · 新地图 · 战役框架

## A. 地形障碍(峡谷岩壁)
godot 原版地图是平面;真实高度差是引擎级改造且偏离原版。取"悬崖的玩法价值"
(封路/卡口/空军自由)而不做高度网格:

- `SkirmishMapDef` 新增 `terrain_walls: &[TerrainWallSpec]`;
  `TerrainWallSpec { start, end, width }`(地图局部坐标线段)。
- 生成:沿线段每 ~1.6m 放一块 kenney 岩石模型(确定性抖动:种子=索引),
  实体带 `TerrainWall + Selectable(radius)`,不可选中不可攻击。
- 阻挡:并入 NavGrid 重建、move_units 静态避障、建筑摆放碰撞三处的
  障碍查询(`Or<(Structure, ResourceNode, TerrainWall)>`)。空军(Air 域)不受影响。

## B. 新地图(3 张,展示卡口战术)
bevy 原创,`godot_path` 用 `bevy://maps/<id>` 作稳定标识(存档/大厅兼容):
1. 峡谷通道 Canyon Pass — 2人 56x56,中央双横墙留左右两口。
2. 交叉火线 Crossfire — 4人 64x64,十字墙分四象限,四个豁口。
3. 环形谷地 Ring Valley — 4人 60x60,环墙四门,中心富矿。

## C. 战役框架(MVP)
数据驱动、零脚本语言——Rust const 任务表:
- `MissionDef { id, 名称/简报(中英), map, player_faction, enemies(阵营+难度),
  starting_resources, victory, triggers }`
- `MissionVictory`: `DestroyAllEnemies` | `SurviveFor(secs)` | `Headquarters`
  (复用胜利条件系统)。失败沿用现有歼灭判定。
- `MissionTrigger::AtTime { at_sec, action }`;
  `MissionAction`: `Message{zh,en}`(战报+语音)| `SpawnWave{team,units,at,attack_to}`
  | `GrantResources{team,ore,crystal}`。
- 运行时:`ActiveMission(Option<&MissionDef>)` 资源 + 触发调度系统 +
  胜利检查系统(SurviveFor 到时直接 HumanVictory)。
- 入口:前端菜单新增 战役 → `AppScreen::CampaignMenu`(任务列表)→ 进对局
  (由 MissionDef 构建 MatchSetupSettings 并置 ActiveMission)。
- 首发 3 关:站稳脚跟(生存 4 分钟,波次进攻)/ 摧毁前哨(斩首)/
  峡谷合围(歼灭困难 AI,中途己方增援触发)。

## 验证
- 墙体:headless 测试(墙两侧寻路必绕行;空军直飞)+ capture 截图。
- 地图:进入每张新图 capture base 不崩 + NavGrid 阻挡断言。
- 战役:headless 每关可开(SurviveFor 到时胜利;波次按时出现)。
