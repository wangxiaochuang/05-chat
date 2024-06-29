# rust template

## 环境设置

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
sqlx database drop -D postgres://postgres:postgres@127.0.0.1/chat
sqlx database create -D postgres://postgres:postgres@127.0.0.1/chat
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
