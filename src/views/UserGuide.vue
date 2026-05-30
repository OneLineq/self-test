<script setup lang="ts">
// ============================================================
// 刷题助手 — 使用说明
// ============================================================
</script>

<template>
  <div style="max-width: 800px; margin: 0 auto; line-height: 1.8">
    <a-typography>
      <a-typography-title :level="3">📝 刷题助手 使用说明</a-typography-title>

      <a-divider />

      <a-typography-title :level="4">一、基本流程</a-typography-title>
      <ol>
        <li><strong>创建题库</strong>：在「题库管理」页面新建题库。</li>
        <li><strong>导入题目</strong>：在题库的「试题管理」页面，通过 Excel 导入题目，或手动逐题添加。</li>
        <li><strong>开始刷题</strong>：在首页选择题库和练习模式，进入练习。</li>
      </ol>

      <a-typography-title :level="4">二、练习模式</a-typography-title>
      <a-descriptions :column="1" bordered size="small">
        <a-descriptions-item label="顺序练习">
          按题库中题目的原始顺序逐题练习。
        </a-descriptions-item>
        <a-descriptions-item label="随机练习">
          将题库中所有题目随机打乱后练习。
        </a-descriptions-item>
        <a-descriptions-item label="错题练习">
          只练习曾经做错的题目，适合针对性巩固。
        </a-descriptions-item>
        <a-descriptions-item label="模拟考试">
          限时 30 分钟，所有题目在一页显示，交卷后显示得分。
        </a-descriptions-item>
      </a-descriptions>

      <a-typography-title :level="4">三、选项颜色说明</a-typography-title>
      <ul>
        <li><span style="color: #52c41a; font-weight: bold">绿色</span> — 正确答案</li>
        <li><span style="color: #f5222d; font-weight: bold">红色</span> — 你选择了错误选项</li>
      </ul>

      <a-typography-title :level="4">四、Excel 导入格式</a-typography-title>
      <p>导入支持 .xlsx、.xls、.xlsb、.ods 格式（.et 格式请先用 WPS 另存为 .xlsx）。</p>
      <p>Excel 文件的第一行为标题行，从第二行开始为题目数据。推荐的列结构：</p>
      <a-table
        :columns="[
          { title: '列', dataIndex: 'col', width: 80 },
          { title: '内容', dataIndex: 'content' },
          { title: '说明', dataIndex: 'desc' },
        ]"
        :data-source="[
          { key: 1, col: 'A', content: '题型', desc: '如 单选/多选/判断/填空（可选）' },
          { key: 2, col: 'B', content: '题干', desc: '必填' },
          { key: 3, col: 'C', content: '选项A', desc: '' },
          { key: 4, col: 'D', content: '选项B', desc: '' },
          { key: 5, col: 'E', content: '选项C', desc: '根据实际选项数列数可变' },
          { key: 6, col: 'F', content: '选项D', desc: '' },
          { key: 7, col: 'G', content: '答案', desc: '单选填 A，多选填 A,C，判断填 A/B' },
          { key: 8, col: 'H', content: '解析', desc: '可选' },
        ]"
        :pagination="false"
        size="small"
        bordered
      />
      <p style="margin-top: 12px; color: #666; font-size: 13px">
        * 导入时会自动识别标题行，也支持手动调整列映射关系。
      </p>

      <a-typography-title :level="4">五、数据存储</a-typography-title>
      <p>
        所有题库、题目和练习记录均存储在本地 SQLite 数据库中，位于应用数据目录下。
        数据不会上传到任何服务器，完全离线可用。
      </p>

      <a-typography-title :level="4">六、快捷键</a-typography-title>
      <ul>
        <li><kbd>←</kbd> / <kbd>→</kbd> 方向键切换上一题/下一题（练习模式）</li>
        <li><kbd>A</kbd> <kbd>B</kbd> <kbd>C</kbd> <kbd>D</kbd> 快速选择对应选项（练习模式）</li>
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
