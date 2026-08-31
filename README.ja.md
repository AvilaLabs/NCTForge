# NCTForge

[![CI](https://github.com/AvilaLabs/NCTForge/actions/workflows/ci.yml/badge.svg)](https://github.com/AvilaLabs/NCTForge/actions/workflows/ci.yml)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Status: early research](https://img.shields.io/badge/status-early_research-orange.svg)](ROADMAP.md)
[![Clinical use: not validated](https://img.shields.io/badge/clinical_use-not_validated-red.svg)](DISCLAIMER.md)

[English](README.md) | **日本語**

NCTForge は、ホウ素中性子捕捉療法（BNCT）の研究および独立検証を目的とした、
輸送コードに依存しない DICOM ネイティブのオープンソース・ワークベンチです。

本プロジェクトは Rust でゼロから開発されています。最初の計算バックエンドとして
OpenMC を採用しますが、OpenMC 固有のシステムにはしません。将来的には、MCNP、
PHITS、OpenPINT などで計算された結果を、共通の物理線量コントラクトを通じて比較・
検証できるようにする計画です。

> [!WARNING]
> NCTForge は初期研究段階のソフトウェアです。現時点では線量計算システムでも、
> 医療機器でもありません。臨床判断、治療計画、患者治療には使用できません。

## 目標

- DICOM CT および RT Structure Set の幾何学情報を厳密に検証する
- BNCT の4つの物理線量成分（ホウ素、窒素、水素反跳、光子）を分離して扱う
- 各ボクセルの統計的不確かさを保持する
- 物理輸送、ホウ素分布、生物学的重み付けを独立して確認できるようにする
- 入力、核データ、計算条件、出力を再現可能な証拠バンドルとして保存する
- OpenMC と外部輸送コードの結果を共通形式で比較する
- CLI、Python API、および egui デスクトップ・ワークベンチから同じ検証済みコアを使用する

## 現在の状況

- 合成 DICOM ベンチマーク `NF-BNCT-001` の CT/RTSTRUCT 幾何学検証は完了しています。
- 3断面連動表示に対応した egui ワークベンチの基本画面を実装しています。
- OpenMC 0.16 用の決定論的入力と核データ証拠チェーンを構築しています。
- BNCT 応答関数の適格性評価を進めています。未解決の核データ問題は失敗として明示され、
  自動的に無視またはゼロに置換されません。
- 粒子輸送の本実行、statepoint の取り込み、生物学的モデル、患者線量計算は未実装です。

開発段階と合格条件については [ロードマップ（英語）](ROADMAP.md) を参照してください。

## 設計上の特徴

```text
DICOM／ケース入力
        |
輸送コードに依存しないケースモデル
        |
輸送アダプター（最初は OpenMC）
        |
4つの物理線量成分＋不確かさ
        |
バージョン管理された生物学的解釈
        |
品質保証、比較、可視化、証拠バンドル
```

NCTForge の中心的な役割は、特定施設の臨床 TPS を置き換えることではなく、研究コード、
輸送コード、施設間で BNCT の計算結果を再現・監査・比較できる公開基盤を提供することです。

## 関連資料

- [英語版 README](README.md)
- [開発ロードマップ](ROADMAP.md)
- [技術ベースライン](docs/research/TECHNICAL_BASELINE.md)
- [アーキテクチャ](ARCHITECTURE.md)
- [免責事項](DISCLAIMER.md)
- [コントリビューションガイド](CONTRIBUTING.md)

日本語での Issue、技術的なフィードバック、用語・文書の改善提案も歓迎します。

## ライセンス

NCTForge は [Apache License 2.0](LICENSE) で公開されています。
