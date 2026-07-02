# 平衡审计(2026-07-02,capture arena / ai-duel)

工具:`capture arena <单位A> <单位B> [数量] [秒]` — headless N v N 对轰(清场至双方指挥中心、
双方同阵营、经济清零防 AI 增援、每秒重贴 attack-move 防 AI 导演改单)。
校准:同兵种镜像局 = 完美同归于尽(0:0),工具无偏。

## 结论

1. **数值移植保真**:抽查 FlameAssaultBuggy / HeavyMachinegunTrooper / GrenadierTrooper /
   FlakRocketTeam / BomberVTOL,hp/伤害/射速/射程/攻击域与 godot MatchConstants.gd 完全一致。
   "喷火车正面打不过机枪兵"(射程 3.0 vs 4.5)是 godot 原版设计(定位=高速追击),不改。
2. **攻击域克制正确**:坦克 5:0 碾步兵;防空队 can_attack_air only、轰炸机 can_attack_ground
   only,均与 godot 一致。
3. **阵营三角(bevy 扩展)宏观成立**:联盟射速(其余阵营 cooldown x1.08)/ 恶魔拆楼 x1.12 /
   混沌减伤 x0.9。单位镜像对轰联盟 5:0(射速滚雪球),但 ai-duel normal vs normal 整局
   **恶魔胜**(7 兵 6 建筑 vs 4/4)——拆楼优势在真实对局补偿了射速差。微观狠、宏观平。
4. **溅射在密集阵型极强**:5 轰炸机齐射(splash 2.0)两秒清空贴脸防空排——godot 同数值下
   同理;竞技场开局即近射程边缘放大了这点。属两版共同特性,非移植缺陷。

## 用法(平衡回归)
改任何战斗数值后跑:镜像局(必须平局)+ 关键克制局(Tank>HMG、AA/轰炸机域限制)+
`ai-duel normal normal`(阵营宏观)。
