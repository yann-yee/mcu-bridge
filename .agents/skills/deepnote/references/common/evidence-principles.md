# DeepNote Common — Evidence Principles

## 1. Claim First

不要先堆资料。先写清楚要确认的 Claim / Question。

## 2. Authority != Truth != Applicability

分别判断：

- Source Authority；
- Claim Status；
- Applicability。

高权威 Source 也可能只适用于旧版本、特定芯片、特定法律辖区或特定实验条件。

## 3. Preserve Conditions

任何关键结论都尽量保留：

- version；
- hardware / software；
- configuration；
- dataset / benchmark；
- legal jurisdiction / date；
- environment；
- exceptions。

## 4. Traceability

关键数字、机制、限制与冲突必须能回到：

- URL / document；
- section / heading；
- table / figure；
- code path / symbol；
- publication / version。

## 5. Conflict Is Information

来源冲突时：

1. 不做平均；
2. 对比版本和条件；
3. 对比 authority；
4. 记录各自 Evidence；
5. 若无法解决，标记 `CONFLICTING`。

## 6. No Evidence, No Fact

使用：

`ⓘ 此处未取得足够可靠证据。`

不要为了完整补猜测。

## 7. Separate Analysis

模型/作者外的解释使用：

- `[分析]`：由现有 Evidence 推导；
- `[推测]`：合理但证据仍不足。

## 8. Decision-Relevant Depth

EVIDENCE_BRIEF 不追求百科；TOPIC_RESEARCH 不追求收集所有来源。

继续研究的标准：

> 新 Evidence 是否可能改变当前 Claim Status、Applicability 或 Decision？

如果不会，停止。