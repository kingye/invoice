# invoice - 轻量级命令行记账系统

基于 Rust 和 SQLite 构建的命令行发票管理工具，支持发票 CRUD、原始凭证关联、从 PDF/XML/OFD 自动导入发票信息、月结/年结报表生成（Excel）、ZIP 归档。

## 安装

需要 Rust 1.70+ 和 C 编译器（用于 SQLite 静态链接）。

```bash
# 克隆
git clone <repo-url>
cd invoice

# 构建
cargo build --release

# 运行测试
cargo test
```

构建产物在 `target/release/invoice`。

## 使用

### 初始化数据库

```bash
invoice init
```

在当前目录下创建 `.invoice/invoice.db`。

### 从文件导入发票

```bash
# 从 PDF 导入
invoice import ./invoice.pdf

# 从 XML 导入
invoice import ./invoice.xml

# 从 OFD 导入
invoice import ./invoice.ofd

# 从 ZIP（含 XML）导入
invoice import ./invoice.zip

# 预览提取结果（不写入数据库）
invoice import ./invoice.pdf --dry-run

# 覆盖提取的字段
invoice import ./invoice.pdf --category 服务 --remark 测试
```

导入时自动根据文件扩展名选择提取器。PDF 文本为空时自动查找同目录同名 `.xml` 或 `.ofd` 文件补充。

### 添加发票

```bash
invoice add \
  --number FP001 \
  --date 2026-04-01 \
  --type 电子发票 \
  --item 技术服务 \
  --amount 1000 \
  --tax-rate 0.06 \
  --tax 60 \
  --total 1060 \
  --seller XX公司 \
  --seller-tax 91110000MA01 \
  --buyer YY公司 \
  --buyer-tax 91310000MB01 \
  --category 服务 \
  --remark 测试发票 \
  --attach ./contract.pdf
```

### 列出发票

```bash
# 列出所有
invoice list

# 按月筛选
invoice list --month 2026-04

# 按年筛选
invoice list --year 2026

# 按分类筛选
invoice list --category 服务
```

### 查看发票详情

```bash
invoice show 1
```

### 编辑发票

```bash
invoice edit 1 --remark 已付款
invoice edit 1 --amount 1200 --total 1272
invoice edit 1 --attach ./receipt.pdf
```

### 删除发票

```bash
invoice delete 1
```

### 月结/年结

```bash
# 月结
invoice close --month 2026-04

# 年结
invoice close --year 2026
```

结账后生成：
- `.invoice/明细表_YYYY-MM.xlsx` - 明细表
- `.invoice/汇总表_YYYY-MM.xlsx` - 汇总表（按分类+发票类型双维度汇总，含加权平均税率）
- `.invoice/close_YYYY-MM.zip` - ZIP 归档

**注意：** 已结账期间的发票不可修改/删除。

### 导出报表（不结账）

```bash
invoice export --month 2026-04 --output ./preview/
```

与 close 类似但不锁定期间，仅生成报表。

### 查看版本

```bash
invoice --version
```

## 数据存储

所有数据存储在当前目录的 `.invoice/` 下：

```
.invoice/
├── invoice.db          # SQLite 数据库
├── data/               # 附件文件
│   └── FP001/
│       └── contract.pdf
├── 明细表_2026-04.xlsx # 月结明细表
├── 汇总表_2026-04.xlsx # 月结汇总表
└── close_2026-04.zip   # 月结归档
```

## 技术架构

| 组件 | 技术 |
|------|------|
| 语言 | Rust |
| 数据库 | SQLite 3 (rusqlite + bundled) |
| Excel 生成 | rust_xlsxwriter |
| PDF 解析 | lopdf |
| XML 解析 | roxmltree |
| ZIP/OFD 解析 | zip crate |
| CLI 解析 | clap |
| 附件校验 | SHA-256 (sha2) |
| 单二进制 | 所有依赖纯 Rust 或静态链接 |

### 模块结构

```
src/
├── main.rs        # 入口
├── cli.rs         # CLI 命令定义和路由
├── db.rs          # SQLite 数据库层
├── models.rs      # 数据模型 (Invoice, Attachment, Closing)
├── import.rs      # 发票导入核心逻辑（格式检测、提取、入库）
├── extract_xml.rs # EInvoice XML 解析
├── extract_pdf.rs # PDF 元数据+文本提取
├── extract_ofd.rs # OFD(ZIP+XML) 解析
├── attachment.rs  # 凭证管理 (文件复制 + SHA-256)
├── report.rs      # Excel 报表生成
├── archive.rs     # ZIP 归档
└── closing.rs     # 月结/年结逻辑
```

## 设计决策

- **PDF 是必须的原始凭证：** `invoice import <pdf>` 是主要入口，XML/OFD 是可选补充
- **PDF 文本为空时自动查找同目录配套 XML/OFD：** 用户无需手动指定，自动提升提取质量
- **优先使用 XML 数据：** XML 字段最完整可靠，PDF 元数据和文本作为降级方案
- **凭证存储：** 复制到 `.invoice/data/` 目录，避免原文件移动/删除导致丢失
- **结账不可逆：** 已结账期间发票不可修改/删除，保证数据完整性
- **`--dry-run` 模式：** 提取结果先预览，确认后再写入，避免错误数据入库
- **税率存储为小数：** 0.06 = 6%，显示时转换为百分比

## License

MIT
