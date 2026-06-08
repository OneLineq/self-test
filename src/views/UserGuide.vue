<script setup lang="ts">
// ============================================================
// 刷题助手 — 使用说明
// ============================================================
import { FileTextOutlined } from '@ant-design/icons-vue'
</script>

<template>
  <div style="max-width: 800px; margin: 0 auto; line-height: 1.8">
    <a-typography>
      <a-typography-title :level="3"><FileTextOutlined style="margin-right: 8px" />刷题助手 使用说明</a-typography-title>

      <a-divider />

      <a-typography-title :level="4">一、基本流程</a-typography-title>
      <ol>
        <li><strong>创建题库</strong>：在「题库管理」页面新建题库，支持重命名和删除。</li>
        <li><strong>添加题目</strong>：进入题库的「试题管理」页面，通过 Excel 批量导入（推荐），或逐题手动添加。</li>
        <li><strong>开始刷题</strong>：通过左侧菜单栏或首页进入练习模式，选择题库即可开始。</li>
        <li><strong>查看记忆</strong>：在「练习记忆」页面查看各题库的练习记录和统计数据。</li>
      </ol>

      <a-typography-title :level="4">二、页面导航</a-typography-title>
      <p>左侧菜单栏包含以下页面入口：</p>
      <ul>
        <li><strong>首页</strong> — 全局统计（题库数、题目数、累计练习、正确率、今日练习、待复习错题数），四种练习模式快捷入口，题库概览卡片</li>
        <li><strong>题库管理</strong> — 创建、重命名、删除题库</li>
        <li><strong>顺序练习 / 随机练习 / 错题练习 / 模拟考试</strong> — 四种练习模式的快捷入口</li>
        <li><strong>练习记忆</strong> — 查看各题库的练习历史记录和统计</li>
        <li><strong>使用说明</strong> — 当前页面</li>
      </ul>

      <a-typography-title :level="4">三、四种练习模式</a-typography-title>
      <a-descriptions :column="1" bordered size="small">
        <a-descriptions-item label="顺序练习">
          按题库中题目的原始添加顺序逐题练习。支持断点续练：退出后再次进入会弹出提示，可选择继续上次进度或重新开始。
        </a-descriptions-item>
        <a-descriptions-item label="随机练习">
          将题库中所有题目随机打乱顺序后练习，适合全面复习。
        </a-descriptions-item>
        <a-descriptions-item label="错题练习">
          只练习曾经做错的题目（练习记录中标记为错误的题），适合针对性巩固薄弱环节。
        </a-descriptions-item>
        <a-descriptions-item label="模拟考试">
          可配置题型数量、考试时长（5–120 分钟），所有题目在同一页显示，有倒计时提醒，交卷后自动评分并显示正确率。
        </a-descriptions-item>
      </a-descriptions>

      <a-typography-title :level="4">四、练习功能详解</a-typography-title>

      <a-typography-title :level="5">4.1 选择题库</a-typography-title>
      <p>
        点击任一练习模式后，会进入选择题库页面。您可以按题型筛选题目（如只练单选题或多选题），也可以限制每类题型的数量，然后点击"开始"进入练习。
      </p>

      <a-typography-title :level="5">4.2 答题操作</a-typography-title>
      <ul>
        <li><strong>单选题</strong>：点击选项后自动提交答案并显示正误。</li>
        <li><strong>多选题</strong>：点击选择多个选项，点击"确认"按钮后提交。</li>
        <li><strong>判断题</strong>：点击"正确"或"错误"按钮提交。</li>
        <li><strong>填空题</strong>：输入答案后按回车或点击提交按钮。</li>
        <li><strong>不定项开关</strong>：练习界面顶部有一个"不定项"开关，开启后所有选择题（单选/判断）都变为多选模式，适合不确定答案数量时使用。</li>
      </ul>

      <a-typography-title :level="5">4.3 练习界面</a-typography-title>
      <ul>
        <li>每题底部显示当前题号和总题数（如 5/20）。</li>
        <li>通过"上一题"/"下一题"按钮或键盘方向键切换题目。</li>
        <li>底部题号条可点击跳转到任意题目，当前题号高亮，已答题目标记为蓝色。</li>
        <li>支持输入题号直接跳转。</li>
        <li><strong>刷题记忆面板</strong>：每道题右侧可展开查看该题的历史练习记录，包括每次的作答、正确率、掌握程度标签（建议复习/已掌握），也支持清除单道题的练习记忆。</li>
      </ul>

      <a-typography-title :level="5">4.4 模拟考试</a-typography-title>
      <ul>
        <li>进入模拟考试后，先从题库中按题型分层抽取题目（可通过滑块调整各类题型的数量）。</li>
        <li>设置考试时长（默认约每题 1.5 分钟，可在 5–120 分钟范围内调整）。</li>
        <li>所有题目在同一页面显示，可上下滚动浏览。</li>
        <li>顶部显示倒计时，时间归零时自动交卷。</li>
        <li>点击"交卷"按钮后自动计算得分并显示正确率。</li>
      </ul>

      <a-typography-title :level="4">五、试题管理</a-typography-title>
      <p>在「题库管理」页面点击某个题库的"管理"按钮，进入试题管理页面。</p>

      <a-typography-title :level="5">5.1 手动添加/编辑</a-typography-title>
      <ul>
        <li><strong>添加题目</strong>：点击"添加题目"按钮，填写题型（单选/多选/判断/填空）、题干、选项（选择题）和答案，可添加解析。</li>
        <li><strong>编辑题目</strong>：点击题目行的编辑图标，修改后保存。</li>
        <li><strong>删除题目</strong>：点击删除图标确认删除。</li>
      </ul>

      <a-typography-title :level="5">5.2 Excel 导入</a-typography-title>
      <p>点击"导入 Excel"按钮，选择文件后进入导入配置界面：</p>
      <ul>
        <li><strong>支持格式</strong>：.xlsx、.xls、.xlsb、.ods（.et 格式请先用 WPS 另存为 .xlsx）。</li>
        <li><strong>工作表选择</strong>：若文件包含多个工作表，可选择要导入的工作表。</li>
        <li><strong>列映射</strong>：系统会自动识别标题行并匹配各列（支持中英文标题），也可手动调整每列对应的字段（题干、题型、选项起始列、选项数量、答案、解析）。</li>
        <li><strong>题型覆盖</strong>：可选择强制将所有导入题目设为指定题型。</li>
        <li><strong>去重策略</strong>：当导入题目与已有题目题干重复时，可选择<em>覆盖</em>（更新）、<em>跳过</em>（保留原有）或<em>追加</em>（全部新增）。</li>
        <li>导入前会预览前 3 行数据，确认无误后执行导入，完成后显示成功/跳过/覆盖/失败的数量统计。</li>
      </ul>

      <a-typography-title :level="5">5.3 Excel 导出</a-typography-title>
      <p>点击"导出 Excel"按钮，选择保存位置，将当前题库所有题目导出为 .xlsx 文件（包含题型、题干、选项、答案、解析列）。</p>

      <a-typography-title :level="5">5.4 批量操作</a-typography-title>
      <ul>
        <li>勾选多个题目后，可批量设置题型，或批量删除。</li>
        <li>点击"清空题库"可一键删除当前题库的所有题目（不可恢复）。</li>
      </ul>

      <a-typography-title :level="4">六、练习记忆</a-typography-title>
      <p>在「练习记忆」页面可以查看全局统计（累计练习次数、正确数、错误数、正确率），以及每个题库的详细练习记录。</p>
      <ul>
        <li>点击题库卡片可展开查看该题库的所有练习记录（按时间倒序排列）。</li>
        <li>每条记录显示：题型标签、题干、你的答案、正确答案、练习模式、作答时间。</li>
        <li>点击"清除记忆"可删除某个题库的所有练习记录并重置题目统计（不可恢复）。</li>
      </ul>

      <a-typography-title :level="4">七、选项颜色说明</a-typography-title>
      <ul>
        <li><span style="color: #52c41a; font-weight: bold">绿色</span> — 正确答案</li>
        <li><span style="color: #f5222d; font-weight: bold">红色</span> — 你选择了错误选项</li>
      </ul>

      <a-typography-title :level="4">八、数据存储</a-typography-title>
      <p>
        所有题库、题目和练习记录均存储在本地 SQLite 数据库中，位于系统应用数据目录下。
        数据不会上传到任何服务器，完全离线可用，无需网络连接。
      </p>

      <a-typography-title :level="4">九、快捷键</a-typography-title>
      <ul>
        <li><kbd>←</kbd> <kbd>↑</kbd> — 上一题（练习模式）</li>
        <li><kbd>→</kbd> <kbd>↓</kbd> — 下一题（练习模式）</li>
        <li><kbd>A</kbd> <kbd>B</kbd> <kbd>C</kbd> <kbd>D</kbd> — 快速选择对应选项（练习 & 考试模式）</li>
        <li><kbd>Enter</kbd> — 确认提交多选 / 填空答案</li>
      </ul>
    </a-typography>
  </div>
</template>

<style scoped>
kbd {
  display: inline-block;
  padding: 2px 6px;
  font-size: 12px;
  font-family: monospace;
  background: #f5f5f5;
  border: 1px solid #d9d9d9;
  border-radius: 3px;
  margin: 0 2px;
}
</style>
