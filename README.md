# rust template

## 环境设置

### 安装 Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 安装 VSCode 插件

- crates: Rust 包管理
- Even Better TOML: TOML 文件支持
- Better Comments: 优化注释显示
- Error Lens: 错误提示优化
- GitLens: Git 增强
- Github Copilot: 代码提示
- indent-rainbow: 缩进显示优化
- Prettier - Code formatter: 代码格式化
- REST client: REST API 调试
- rust-analyzer: Rust 语言支持
- Rust Test lens: Rust 测试支持
- Rust Test Explorer: Rust 测试概览
- TODO Highlight: TODO 高亮
- vscode-icons: 图标优化
- YAML: YAML 文件支持

### 安装 cargo generate

cargo generate 是一个用于生成项目模板的工具。它可以使用已有的 github repo 作为模版生成新的项目。

```bash
cargo install cargo-generate
```

新的项目使用 `wangxiaochuang/rustpl` 模版生成基本的代码：

```bash
cargo generate wangxiaochuang/rustpl
```

### 安装 pre-commit

pre-commit 是一个代码检查工具，可以在提交代码前进行代码检查。

```bash
pipx install pre-commit
# 实际应用pre-commit
pre-commit install
```

### 安装 Cargo deny

Cargo deny 是一个 Cargo 插件，可以用于检查依赖的安全性。

```bash
cargo install --locked cargo-deny

# 获取最新的规则
cargo deny fetch
```

### 安装 typos

typos 是一个拼写检查工具。

```bash
cargo install typos-cli
```

### 安装 git cliff

git cliff 是一个生成 changelog 的工具。

```bash
cargo install git-cliff
```

安装完后修改postprocessors,设置正确的的repo

### 安装 cargo nextest

cargo nextest 是一个 Rust 增强测试工具。

```bash
cargo install cargo-nextest --locked

# 运行测试
cargo nextest run
```

### 代码commit规范

commit message 格式
```txt
<type>(<scope>): <subject>

type: 必填,允许的标识
    feature: 新功能
    fix/to: 修复bug,fix为最后一次提交,to为问题解决前的多次提交
    docs: 文档更新
    refactor: 重构,既不新增功能,也不修改bug
    perf: 优化,比如性能体验等
    test: 增加测试
    chore: 构建工具或辅助工具的变动
    merge: 代码合并
scope: 可选,说明commit影响的范围,名称自定义,影响多个可使用*
subject: commit目的的简短描述,不超过50个字符
```

例如下面的提交样例

```txt
fix(DAO): 用户查询缺少username属性
feature(Controller): 用户查询接口开发
```

## postgres

### install

```
pip install pgcli
cargo install sqlx-cli --no-default-features --features rustls --features postgres
```

进入指定库

```sh
pgcli -h 127.0.0.1 -U postgres chat
```

创建、删除库

```sh
sqlx database create -D postgres://postgres:postgres@127.0.0.1/chat
sqlx database drop -D postgres://postgres:postgres@127.0.0.1/chat
```

创建迁移文件

```sh
sqlx migrate add initial

```

执行迁移文件

```sh
# 执行后，会在chat下创建一个名为_sqlx_migrations的表，记录了迁移的内容，多次执行不会有变化，如果文件改变会报错
# echo DATABASE_URL=postgres://postgres:postgres@127.0.0.1/chat > .env
# sqlx migrate run
sqlx migrate run -D postgres://postgres:postgres@127.0.0.1/chat
```
