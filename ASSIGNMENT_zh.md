# 區塊鏈門檻簽章錢包實作測驗 (FROST & HD Wallet on Solana)

歡迎來到本次實作測驗！我們很期待在這個充滿挑戰的作業中看到你的創意與技術實力。

## 🎯 測驗目標

這份測驗的設計初衷，是為了評估你如何利用 **AI Agent（如 Claude Code, Codex, OpenCode, Cursor, Windsurf, Antigravity 等）** 來快速建構複雜系統，同時觀察你對密碼學協議與系統架構切分的理解。

請實作一個最小可運作的 **2-of-2 TSS Solana Wallet Demo System**。測驗範圍涵蓋現代密碼學（FROST 門檻簽章）、Rust 後端開發、前端介面設計，以及跨節點狀態管理的綜合應用。這題的目的「不在於」打造一個 production-ready 的 MPC 基礎設施，而是希望你能將核心的資料流、協議流程、狀態管理與前後端整合做到位，並且**強烈建議你充分展現如何引導 AI 來解決專業領域的問題**。

本題的重點在於**功能與可觀察行為的驗收**，而非要求你證明某一套固定的密碼學推導形式。只要你確實使用了 `frost-ed25519` 與 `hd-wallet` 的 non-hardened Edwards derivation，完成一次 root-level 2-of-2 DKG 後，能在前端持續推導 (derive) 多個錢包地址，並讓節點使用同一組 root key share 對不同 wallet index 的交易完成門檻簽章，即符合我們的期待。

本作業採用「**指定技術 + 指定可觀察行為驗收**」：除了明確列出的技術限制、協議邊界與環境條件外，內部的抽象層、API 設計、資料庫綱要設計、狀態流轉與實作細節，都歡迎你自由發揮。你不需要拘泥於固定做法，只要最終的系統行為符合驗收條件即可。

---

## 🛠️ 題目範圍

- **公鏈環境**：Solana (Devnet)
- **曲線 / 簽章演算法**：Ed25519 / FROST
- **門檻設定 (Threshold)**：2-of-2
- **Wallet Derivation**：Non-hardened Edwards derivation for Ed25519（不要求與現行主流 Solana wallet derivation 完全相容）
- **RPC Endpoint**：請統一使用 `https://api.devnet.solana.com`
- **資金來源 (Funding)**：驗收時，審閱者會自行將 Devnet SOL 轉入衍生出的錢包地址；若你能額外提供 airdrop 索取測試幣功能將視為加分，但這並非必要條件。
- **交易類型**：系統至少需支援基礎的 Solana Devnet SOL 轉帳 (Transfer)

## 💻 技術要求

為了讓環境保持一致，請使用以下的技術：

- **Backend:** Rust `1.94.0` + `axum` `0.8.8` + `snafu` `0.8.7` + `sqlx` `0.8.6` + PostgreSQL `18`
- **Frontend:** Next.js `16` + React `19` + TypeScript `5.9`
- **Solana:** `solana-client` `3.1.8` / `solana-sdk` `3.0.0`
- **Core crypto:** [`frost-ed25519` `2.1.0`](https://github.com/ZcashFoundation/frost), [`hd-wallet` `0.6.1`](https://docs.rs/hd-wallet/latest/hd_wallet/)（使用 `Edwards` non-hardened derivation）

你可以自由引入其他必要的套件，但請盡量保持系統架構簡潔、程式碼具可讀性且易於執行。你可以自行決定如何銜接 FROST root share 與 `hd-wallet` 的 derivation 流程；我們不限定內部的數學抽象實作，主要根據最終系統行為是否符合需求進行驗收。

---

## 📐 系統架構與元件職責

為了模擬真實的 TSS (Threshold Signature Scheme) 環境，系統分為以下幾個主要元件：

### 1. TSS Coordinator

主要負責居中協調的角色：

- **協調 DKG 流程**：轉發來自前端的觸發指令給指定的 Node，並統整、保存各階段的交換資料。
- **協調 Signing 流程**：接收前端的待簽名請求，並發送 Signing Round 的啟動指令給指定 Node，隨後收集 Commitments 與 Signature Shares。
- **聚合與廣播**：將收集到的 Signature Shares 聚合為最終的 Aggregated Signature，並構建完整交易 Broadcast 到 Solana Devnet。
- **一般錢包操作**：處理 Wallet address 的推導、餘額查詢等應用層邏輯。

### 2. TSS Nodes (Node A & Node B)

每個 Node 代表一個獨立的簽署方 (Signer)，負責執行所有的密碼學運算。**私鑰 (Share) 請務必保留在 Node 內部，絕不可離開**：

- 執行 DKG 各個回合的密碼學運算。
- 執行 Signing 的各個回合：產生 Nonces/Commitments、推導 Child Key Share、並計算出 Signature Share。
- **On-the-fly Derivation（核心要求）**：節點平時只需安全地保存 DKG 產出的 Root Share。系統應設計為當需要產生新的錢包地址以供前端展示時，**不需要 TSS Nodes 彼此進行互動運算**，而是根據 wallet index 持續進行推導；在參與 Signing 階段時，節點能夠基於同一組 Root Share，於記憶體中即時無縫地推導出對應 wallet index 的簽章材料，從而順利完成衍生錢包的門檻簽章。

### 3. Frontend (狀態展示與協議驅動)

前端是整個系統的唯一互動介面，透過呼叫 Coordinator API 來驅動所有流程。我們希望前端能夠**將 FROST 的多回合 (Multi-round) 互動特徵視覺化**，讓使用者能夠清楚看到並逐步觸發各個節點與協調者之間的協議狀態。

---

## 💾 資料持久化

- 系統請使用**單一 PostgreSQL instance**（各個服務可以切分不同的 database 或 schema 來使用）。
- Coordinator 與各 Node 需各自負責持久化其運作所需的狀態資料。**具體的 Data Schema 設計與持久化策略由你來決定**，只要確保最終的系統行為符合預期即可。
- **基本要求**：當系統重啟後，先前已完成的 DKG 結果、已推導的錢包列表以及交易紀錄，都必須能被正確還原，不可遺失。

---

## ✅ 核心使用情境與驗收條件

以下是我們會實際操作並驗收的三大核心流程：

### A. DKG (分散式金鑰生成) 流程

前端介面必須提供「**逐步觸發 / 獨立觸發**」的 DKG 互動方式。每個節點在每一回合 (Round) 的行為，都必須有對應的獨立操作按鈕或等價控制元件；**請勿設計單一的 `Run All` 按鈕將整個複雜的 DKG 流程隱藏在幕後自動完成**。

**驗收條件：**

1. 使用者可以單獨觸發 Node A 與 Node B 分別執行 DKG 的每一個階段（Round 1 / Round 2 / Round 3）。
2. DKG 順利完成後，系統應準備好後續 Wallet Derivation 所需的 root-level material，使得前端能無縫持續建立出多個衍生的錢包。在實作上，務必確認你使用了 `frost-ed25519` 與 `hd-wallet` 的 non-hardened Edwards derivation。
3. 前端必須能清晰地展示出每一個 Node 在每個階段的操作完成狀態、總體的 DKG 進度條，以及最終生成的 Master Public Key (Base58 格式)。

### B. Wallet Derivation (錢包推導)

利用 DKG 產生的 root-level material 作為基礎，執行 Non-hardened Edwards Derivation 來派生子錢包地址。

**驗收條件：**

1. 畫面具備一個「Create Wallet」按鈕，每點擊一次，系統就會自動使用下一個順序遞增的 index 推導出全新的錢包。
2. 前端需能列出**所有已推導的錢包清單**，清單中至少包含「Wallet Index」與對應的「Solana Address (Base58)」，並提供查詢或直接顯示該地址於 Devnet 上的 SOL 餘額。
3. 衍生錢包地址的建立，不應引發或依賴 TSS Nodes 之間的進一步網路互動；這裡旨在驗證：同一組 root key share 就能涵蓋、對應多組不同的 wallet index。
4. 使用者可以在錢包清單中**選擇任何一個錢包作為準備發送轉帳的 Sender (發款方)**。

### C. 門檻簽章與交易轉帳 (FROST Signing Flow)

與 DKG 類似，前端需要將一次轉帳的內部過程拆解為多個可**獨立觸發**的操作步驟。每個節點在 Signing 的每一回合都必須要有對應的操作元件；**請勿設計單一的 `Sign & Send` 按鈕一鍵完成**。

**驗收條件：**

1. **建立交易**：使用者需從錢包列表中選擇 sender wallet，接著輸入接收方目標地址與轉帳金額。Coordinator 接收請求後會生成一筆獨立且具可識別性的**待簽名請求 (Signing Request)**。
2. **待簽名請求列表**：前端需列舉目前所有的待簽名請求，包含但不限於：sender wallet、目標地址、金額、請求建立時間以及當前狀態。介面需能讓使用者輕易辨識不同請求的差異，並點選其中一筆進入後續的簽章流程。
3. **Signing Round 1**：使用者可單獨觸發 Node A 或 Node B 針對指定的待簽名請求執行第一階段運算。
4. **Signing Round 2**：使用者可單獨觸發 Node A 或 Node B 針對指定的待簽名請求執行第二階段運算。在此階段，節點必須確保是基於同一組 Root Share 與對應的 wallet index，來完成衍生錢包層級的門檻簽章。
5. **聚合與廣播 (Aggregation & Broadcast)**：當簽章備齊，使用者可觸發 Coordinator 對該請求執行簽章聚合，組裝成合規的 Solana 交易並廣播至 Solana Devnet。
6. **狀態視覺化：**
   - 能夠以易懂的方式呈現 Signing Request 的狀態演進（例如：待簽名 → 簽章中 → 已廣播 → 已確認 → 失敗），當中「已確認」狀態請以 Solana 網路的回報 `confirmed` 狀態為準。
   - 能夠個別顯示 Node A / Node B 在該筆 request 中，每個簽章階段的完成燈號或狀態。
   - 廣播成功後，提供指向該筆交易的 Solana Explorer Transaction Hash 連結。

> **⏰ 期待的完成時間估算：約一週（7 天）。** <br/> 請不用倍感壓力，合理安排你的投入時間，並建議優先確保核心的 DKG 與 Signing 流程邏輯正確運作。

---

## 📋 提交與評分維度

本次面試作業的評分架構如下：

| 評量維度                      | 佔比    | 說明                                                                                                                                                                                |
| ----------------------------- | ------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **驗收條件達成**              | **40%** | DKG、Wallet Derivation、Signing & Broadcast 這三大核心流程是否能正確走通，且前端是否符合逐步觸發與分層狀態展示的要求。<br/> **⚠️ 此為及格門檻：功能無法順利驗收的流程將不予計分。** |
| **AI 開發歷程與問題解決能力** | **40%** | 我們非常看重你如何與 AI 協作，這包含你引導 AI 走過複雜密碼學工程挑戰的完整經過（詳細要求請見下方說明）。                                                                            |
| **工程品質與加分項**          | **20%** | 安全性防護（如：input validation、完善的 error handling、nonce 防護處理）、程式碼結構與品質、測試覆蓋程度、系統架構設計的周全性、文件撰寫品質，以及任何超出基本設計的加值功能。     |

### 1. AI 開發歷程與文件 (佔評分 40%)

在現今的工作環境中，善用 AI 是極大的優勢。本次測驗**非常重視**你如何引導並與 AI 配對編程來完成這個具備挑戰性的專案。在開發過程中，請務必養成記錄的習慣。

- 可接受的繳交形式多元：包含但不限於 Prompt 歷史紀錄、對話逐字匯出、重點截圖、Markdown 整理文件、架構決策日誌 (Decision Log)，或是任何能幫助我們理解你與 AI 互動過程的素材。
- 當開發碰壁，或是 AI 給出方向錯誤的建議時，**請特別記錄你是如何修正與引導它的**。
- 如果對話中涉及敏感資訊或是難以消化的長文，你可以適度地進行清理與精簡 (sanitize)，但請務必保留那些足以展現你解題思考脈絡的關鍵對話。

我們主要想從中了解：

- **未知領域探索與快速學習**：面對不熟悉的技術（如區塊鏈、Rust 或是 FROST 密碼學），你如何利用 AI 作為學習加速器，快速掌握新知並落地實踐？
- **複雜問題拆解能力**：面對一個大架構，你如何將其切割成 AI 能有效消化並給出好答案的子任務？
- **Prompt 規劃與上下文管理**：你的提示詞是否夠精確？是否提供了充足且必要的上下文脈絡來引導 AI 到達正確的結果？
- **糾偏與除錯策略**：當 AI 產出「幻覺」或程式碼含有 bug 時，你的除錯策略與思路為何？如何引導 AI 自我修正？
- **主見與技術決策**：在專案的哪幾個關鍵分水嶺，是你跳出來做了人類工程師的專業判斷，而不是照單全收 AI 的建議？

### 2. 運行指南 (README & Docker Compose)

為了加速雙方的交流與驗收，請提供清晰的本地啟動方式：

- **無需上雲部署**: 你只要提供一份說明清楚的 `README.md`，引導我們如何在本地端 (Local) 順利編譯與運行整個系統即可。
- **Docker Compose**: **必須** 包含一份 `docker-compose.yml`。我們希望能透過簡單的 `docker compose up` 指令，就一鍵啟動包含 Frontend、Coordinator、Node A、Node B 與 PostgreSQL（單一 instance 即可）的完整生態。
- 系統應將 RPC Endpoint 設定為可透過環境變數覆蓋（例如 `SOLANA_RPC_URL`），並以 `https://api.devnet.solana.com` 作為預設值，且需在 `docker-compose.yml` 或 `.env.example` 中明確標示。同時，請在 README 中說明審閱者可在哪裡查閱衍生出的錢包地址，以便手動發送 Devnet SOL 作為測試資金。若你額外接了水龍頭 (Faucet) 服務，也請一併介紹！

### 3. 自動化測試體驗

雖然我們強調手動操作的驗收與 AI 的協作過程，但優秀的軟體工程仍離不開自動化測試。
非常歡迎你發揮所長，撰寫 Unit tests 或是 Integration tests！在此領域，涵蓋範圍與測試的戰略由你全權規劃。我們會從這個環節，評估你對於系統脆弱關鍵路徑（Critical Path）的保護意識，以及你對於整體工程品質的深刻見解。

再次感謝你投入時間，祝你能夠享受解題的過程、並取得好成績！ 🎉
