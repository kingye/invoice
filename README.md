# invoice - 轻量级命令行记账系统

基于 Zig 语言和 SQLite 构建的命令行发票管理工具，支持发票 CRUD、原始凭证关联、月结/年结报表生成（Excel）、ZIP 归档。

## 安装

需要 Zig 0.13.0+、zlib 开发库、zip 命令行工具、curl、unzip。

```bash
# 克隆（含子模块）
git clone --recurse-submodules <repo-url>

# 安装依赖 (Debian/Ubuntu)
sudo apt install zlib1g-dev zip curl

# 构建（自动下载 SQLite amalgamation）
zig build

# 运行测试
zig build test
```

构建产物在 `zig-out/bin/invoice`。

## 使用

### 初始化数据库

```bash
invoice init
```

在当前目录下创建 `.invoice/invoice.db`。

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
| 语言 | Zig 0.13.0 |
| 数据库 | SQLite 3 (静态链接) |
| Excel 生成 | libxlsxwriter (静态链接) |
| ZIP 归档 | 系统 zip 命令 |
| 附件校验 | SHA-256 |
| 单二进制 | 所有 C 依赖静态链接 |

### 模块结构

```
src/
├── main.zig        # 入口 + 命令路由
├── cli.zig         # CLI 参数解析
├── db.zig          # SQLite 数据库层
├── models.zig      # 数据模型 (Invoice, Attachment, Closing)
├── attachment.zig  # 凭证管理 (文件复制 + SHA-256)
├── report.zig      # Excel 报表生成
├── closing.zig     # 月结/年结逻辑
└── archive.zig     # ZIP 归档
```

## 设计决策

- **凭证存储：** 复制到 `.invoice/data/` 目录，避免原文件移动/删除导致丢失
- **数据库抽象：** `db.zig` 封装所有 SQL 操作，未来替换数据库只需修改此模块
- **结账不可逆：** 已结账期间发票不可修改/删除，保证数据完整性
- **发票类型自由文本：** 非枚举值，方便用户自定义扩展
- **税率存储为小数：** 0.06 = 6%，显示时转换为百分比

## License

MIT
