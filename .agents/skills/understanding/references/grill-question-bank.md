# Decision Question Guide V2

本文件不是“必须问完的问题库”，而是帮助 `understanding` 找到真正会改变 Goal Model 的未知。

## 先判断：这是用户问题还是证据问题？

优先级：

1. 既有 `context.md` / Goal / design 是否已经回答；
2. 代码、测试、配置是否能回答 —— 能则交给 Explorer；
3. 外部领域事实是否需要 `teach-me`；
4. 只有用户意图、业务取舍、偏好与不可从证据推导的决策才询问用户。

## 好问题必须改变决策

问之前先回答：

> 用户不同答案会不会改变 Desired Outcome、What Good Looks Like、Invariant、Non-Goal 或高层 Solution Direction？

如果不会，不要在 understanding 阶段问。

## 目标类问题

- 完成后你真正希望什么状态变得不同？
- 如果只能保留一个结果，哪个结果最重要？
- 这个需求是在解决根因，还是只是当前想到的界面/技术手段？

## 质量取舍类问题

只问当前确实存在冲突的维度，例如：

- latency vs consistency；
- simplicity vs extensibility；
- memory usage vs throughput；
- compatibility vs breaking redesign；
- automation vs human control。

不要把所有质量维度逐个 Grill 一遍。

## 边界类问题

- 哪些行为必须保持兼容？
- 哪些相邻问题明确不在本次范围？
- 哪个失败结果是不能接受的？
- 哪项长期架构方向不能为了局部快速实现而破坏？

## 风险类问题

先让 Explorer 找证据。只有证据无法决定且风险高时再问用户：

- 数据丢失/迁移语义；
- 权限/所有权；
- 钱/计费；
- 对外协议兼容；
- 不可逆硬件或部署行为。

## 提问格式

一次只处理一个真正的决策点：

```text
Decision:
为什么这个问题影响目标：
Option A:
Option B:
推荐：
推荐理由与代价：
```

用户确认的是决策结果，不需要参与代码事实调查。

## 禁止模式

- 为了显示严谨而问 20 个边缘条件；
- 能查代码却问用户“项目现在怎么实现”；
- 在还没确认 Goal 时追问函数/API 细节；
- 把自己的推荐包装成“唯一正确答案”；
- 因为完美信息不存在而永远不进入 `GOAL_READY_WITH_ASSUMPTIONS`。
