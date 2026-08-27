<script setup lang="ts">
// ============================================================
// 理论训练考核系统 — 使用说明（含 Word 式查找）
// ============================================================
import { ref, watch, onMounted, onBeforeUnmount, nextTick } from 'vue'
import {
  FileTextOutlined,
  SearchOutlined,
  UpOutlined,
  DownOutlined,
  CloseOutlined,
} from '@ant-design/icons-vue'

const contentRef = ref<HTMLElement | null>(null)
const findInputRef = ref<{ focus: () => void } | null>(null)
const query = ref('')
const findOpen = ref(false)
const hits = ref<HTMLElement[]>([])
const currentIndex = ref(-1)

const HIT_CLASS = 'guide-find-hit'
const ACTIVE_CLASS = 'guide-find-hit--active'

function clearHighlights() {
  const root = contentRef.value
  if (!root) return
  root.querySelectorAll(`mark.${HIT_CLASS}`).forEach((mark) => {
    const parent = mark.parentNode
    if (!parent) return
    parent.replaceChild(document.createTextNode(mark.textContent || ''), mark)
    parent.normalize()
  })
  hits.value = []
  currentIndex.value = -1
}

function collectTextNodes(root: HTMLElement): Text[] {
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT)
  const nodes: Text[] = []
  while (walker.nextNode()) {
    const n = walker.currentNode as Text
    if (!n.textContent?.trim()) continue
    // 跳过已在 mark 内的（清理后不应有）
    if (n.parentElement?.closest(`mark.${HIT_CLASS}`)) continue
    nodes.push(n)
  }
  return nodes
}

function highlightAll(raw: string): HTMLElement[] {
  const root = contentRef.value
  if (!root) return []
  clearHighlights()
  const q = raw.trim()
  if (!q) return []

  const lowerQ = q.toLowerCase()
  const found: HTMLElement[] = []
  // 先收集再改 DOM，避免 TreeWalker 失效
  const nodes = collectTextNodes(root)

  for (const textNode of nodes) {
    const text = textNode.textContent || ''
    const lower = text.toLowerCase()
    let start = 0
    const pieces: Array<string | HTMLElement> = []
    let matched = false

    while (start < text.length) {
      const i = lower.indexOf(lowerQ, start)
      if (i === -1) {
        if (matched) pieces.push(text.slice(start))
        break
      }
      matched = true
      if (i > start) pieces.push(text.slice(start, i))
      const mark = document.createElement('mark')
      mark.className = HIT_CLASS
      mark.textContent = text.slice(i, i + q.length)
      pieces.push(mark)
      found.push(mark)
      start = i + q.length
    }

    if (!matched) continue
    const frag = document.createDocumentFragment()
    for (const p of pieces) {
      frag.appendChild(typeof p === 'string' ? document.createTextNode(p) : p)
    }
    textNode.parentNode?.replaceChild(frag, textNode)
  }

  return found
}

function setActive(index: number) {
  hits.value.forEach((el) => el.classList.remove(ACTIVE_CLASS))
  if (index < 0 || index >= hits.value.length) {
    currentIndex.value = -1
    return
  }
  currentIndex.value = index
  const el = hits.value[index]
  el.classList.add(ACTIVE_CLASS)
  el.scrollIntoView({ behavior: 'smooth', block: 'center' })
}

function runSearch(resetIndex = true) {
  const list = highlightAll(query.value)
  hits.value = list
  if (list.length === 0) {
    currentIndex.value = -1
    return
  }
  setActive(resetIndex ? 0 : Math.min(currentIndex.value, list.length - 1))
}

function goNext() {
  if (hits.value.length === 0) {
    runSearch()
    return
  }
  setActive((currentIndex.value + 1) % hits.value.length)
}

function goPrev() {
  if (hits.value.length === 0) {
    runSearch()
    return
  }
  const n = hits.value.length
  setActive((currentIndex.value - 1 + n) % n)
}

function openFind() {
  findOpen.value = true
  nextTick(() => {
    findInputRef.value?.focus()
  })
}

function closeFind() {
  findOpen.value = false
  query.value = ''
  clearHighlights()
}

function onFindKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter') {
    e.preventDefault()
    if (e.shiftKey) goPrev()
    else goNext()
  } else if (e.key === 'Escape') {
    e.preventDefault()
    closeFind()
  }
}

function onGlobalKeydown(e: KeyboardEvent) {
  const isFind = (e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'f'
  if (isFind) {
    e.preventDefault()
    openFind()
    return
  }
  if (!findOpen.value) return
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'g') {
    e.preventDefault()
    if (e.shiftKey) goPrev()
    else goNext()
  }
}

watch(query, () => {
  if (!findOpen.value) return
  runSearch(true)
})

onMounted(() => {
  window.addEventListener('keydown', onGlobalKeydown)
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onGlobalKeydown)
  clearHighlights()
})
</script>

<template>
  <div class="guide-page">
    <!-- Word 式查找栏 -->
    <div v-if="findOpen" class="guide-find-bar">
      <SearchOutlined class="guide-find-icon" />
      <a-input
        ref="findInputRef"
        v-model:value="query"
        placeholder="查找…"
        allow-clear
        size="small"
        style="width: 220px"
        @keydown="onFindKeydown"
      />
      <span class="guide-find-count">
        <template v-if="!query.trim()">输入关键词</template>
        <template v-else-if="hits.length === 0">无结果</template>
        <template v-else>{{ currentIndex + 1 }} / {{ hits.length }}</template>
      </span>
      <a-button size="small" :disabled="hits.length === 0" title="上一个 (Shift+Enter)" @click="goPrev">
        <UpOutlined />
      </a-button>
      <a-button size="small" :disabled="hits.length === 0" title="下一个 (Enter)" @click="goNext">
        <DownOutlined />
      </a-button>
      <a-button size="small" type="text" title="关闭 (Esc)" @click="closeFind">
        <CloseOutlined />
      </a-button>
    </div>
    <div v-else class="guide-find-trigger">
      <a-button size="small" @click="openFind">
        <SearchOutlined /> 查找
      </a-button>
      <span class="guide-find-hint">Ctrl+F</span>
    </div>

    <div ref="contentRef" class="guide-content">
      <a-typography>
        <a-typography-title :level="3"><FileTextOutlined style="margin-right: 8px" />理论训练考核系统 使用说明</a-typography-title>

        <a-divider />

        <a-typography-title :level="4">一、基本流程</a-typography-title>
        <ol>
          <li><strong>创建题库</strong>：在「题库管理」页面新建题库，支持重命名和删除。可按名称搜索题库。</li>
          <li><strong>添加题目</strong>：进入题库的「题目管理」页面，通过 Excel 批量导入（推荐），或逐题手动添加。</li>
          <li><strong>开始刷题</strong>：通过左侧菜单栏或数据看板进入练习模式，选择题库即可开始。</li>
          <li><strong>查看记忆</strong>：在「练习记忆」页面查看各题库的练习记录和统计数据。</li>
          <li><strong>备份数据</strong>：在「数据管理」页面导出 .db 备份，也可导入或合并其他备份。</li>
        </ol>

        <a-typography-title :level="4">二、页面导航</a-typography-title>
        <p>左侧菜单栏包含以下页面入口：</p>
        <ul>
          <li><strong>数据看板</strong> — 全局统计（题库数、题目数、累计练习、正确率、今日练习、待复习错题数），练习模式快捷入口，题库概览（可按名称搜索；卡片上的「题目管理」进入该题库题目列表）</li>
          <li><strong>题库管理</strong> — 创建、重命名、删除题库，按名称搜索；「题目管理」进入该题库的题目列表</li>
          <li><strong>数据管理</strong> — 查看当前数据库、导出备份、导入替换或合并</li>
          <li><strong>顺序练习 / 随机练习 / 错题练习 / 主题练习 / 模拟考试</strong> — 练习模式快捷入口</li>
          <li><strong>练习记忆</strong> — 查看各题库的练习历史记录和统计</li>
          <li><strong>使用说明</strong> — 当前页面</li>
          <li><strong>设置</strong> — 侧栏底部，练习默认开关、功能入口与版本信息</li>
        </ul>

        <a-typography-title :level="4">三、练习模式</a-typography-title>
        <a-descriptions :column="1" bordered size="small">
          <a-descriptions-item label="顺序练习">
            按题库中题目的原始添加顺序逐题练习。支持断点续练：退出后再次进入会弹出提示，可选择继续上次进度或重新开始。
          </a-descriptions-item>
          <a-descriptions-item label="随机练习">
            将题库中所有题目随机打乱顺序后练习，适合全面复习。
          </a-descriptions-item>
          <a-descriptions-item label="错题练习">
            只练习曾经做错的题目（含手动标记的错题）。默认按「练习次数少 → 正确率低」排序，优先练生疏题。
          </a-descriptions-item>
          <a-descriptions-item label="主题练习">
            先按关键词或正则筛选题干与选项，再选择顺序、随机或错题方式练习。适合按考点集中刷题。
          </a-descriptions-item>
          <a-descriptions-item label="模拟考试">
            可按题型抽取数量、设置考试时长，所有题目在同一页显示，有倒计时，交卷后自动评分并回顾错题。
          </a-descriptions-item>
        </a-descriptions>

        <a-typography-title :level="4">四、练习功能详解</a-typography-title>

        <a-typography-title :level="5">4.1 选择题库</a-typography-title>
        <p>
          点击任一练习模式后，会进入选择题库页面。可按名称搜索题库。进入后可按题型筛选（如只练单选题或多选题），也可限制每类题型的数量，然后点击「开始」。
        </p>

        <a-typography-title :level="5">4.2 主题练习</a-typography-title>
        <p>选择题库后进入筛选页，设置条件并预览命中题数，再开始练习：</p>
        <ul>
          <li><strong>关键词</strong>：用空格或逗号分隔多个词，匹配范围包括题干和选项。示例：<code>网络 路由</code> 或 <code>网络,路由</code>。</li>
          <li><strong>任一命中</strong>：命中任意一个关键词即可；<strong>全部命中</strong>：每个关键词都要出现。</li>
          <li><strong>正则表达式</strong>：开启后，每个词作为一条正则。写法错误时页面会提示，无法开始。</li>
          <li><strong>练习方式</strong>：顺序（按题库原序）、随机（打乱匹配题）、错题（仅在错题集中匹配）。</li>
          <li>主题顺序练习不沿用全库断点续练，每次从筛选结果的第一题开始。</li>
        </ul>

        <a-typography-title :level="5">4.3 答题操作</a-typography-title>
        <ul>
          <li><strong>单选题</strong>：点击选项后自动提交答案并显示正误。答对自动跳转下一题，答错停留查看解析。</li>
          <li><strong>多选题</strong>：点击选择多个选项，点击「确认」后提交。答对自动跳转下一题，答错停留。</li>
          <li><strong>判断题</strong>：点击「正确」或「错误」按钮提交。</li>
          <li><strong>填空题</strong>：输入答案后按回车或点击提交按钮。</li>
        </ul>

        <a-typography-title :level="5">4.4 练习界面</a-typography-title>
        <ul>
          <li>顶部显示当前题号和总题数（如 5/20），主题练习会带「主题筛选」标签。</li>
          <li>通过「上一题」/「下一题」按钮或键盘方向键切换题目。</li>
          <li>底部题号条可点击跳转到任意题目，当前题号高亮，已答题目标记为颜色（绿色正确、红色错误）。</li>
          <li>支持输入题号直接跳转。</li>
          <li><strong>更多菜单</strong>中的当场开关（默认值在「设置」里改）：
            <ul>
              <li><strong>不定项</strong>：选择题不区分单选/多选，可自由选一项或多项后提交。</li>
              <li><strong>打乱答案顺序</strong>：多选判分不区分选项顺序（A,B 与 B,A 都算对）。</li>
              <li><strong>打乱选项顺序</strong>：每道题的选项随机排列，避免背位置。</li>
            </ul>
          </li>
          <li><strong>标记为错题</strong>：作答错误后，底部按钮会立刻变为「移出错题集」。也可在答对后手动点「标记为错题」加入错题集。</li>
          <li><strong>移出错题集</strong>：错题练习时可将已掌握的题目移出。</li>
          <li><strong>刷题记忆面板</strong>：每道题上方可展开查看该题的历史作答、正确率、掌握程度（建议复习/已掌握），也可清除单道题记忆。</li>
        </ul>

        <a-typography-title :level="5">4.5 模拟考试</a-typography-title>
        <ul>
          <li>进入后先按题型抽取：用滑块调整各类数量。未设置题型的题目归入「未分类」，可单独抽取。</li>
          <li>考试时长可用滑块或直接输入分钟数，范围 5–480 分钟（最长 8 小时）。默认按题量估算（约每 5 题 1.5 分钟，最少 5 分钟）。</li>
          <li>可同时开启不定项、打乱答案顺序、打乱选项顺序。开启不定项时会打乱题目顺序，避免按题型成块出现。</li>
          <li>所有题目在同一页显示，可上下滚动。顶部倒计时归零时自动交卷。</li>
          <li>点击「交卷」后自动计分并显示正确率。</li>
          <li><strong>错题回顾</strong>：交卷后逐题展示正误、选项高亮（绿色=正确答案、红色=选错的选项）、你的答案与正确答案对比，以及解析。</li>
        </ul>

        <a-typography-title :level="4">五、题目管理</a-typography-title>
        <p>
          可从「题库管理」点击某个题库的「题目管理」，或从「数据看板」题库概览进入。页面左上角「返回」会回到进入前的页面（看板或题库管理）。
        </p>

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

        <a-typography-title :level="5">5.4 搜索与筛选</a-typography-title>
        <ul>
          <li>顶部搜索框可按<strong>题干或选项</strong>关键字过滤题目。</li>
          <li>还可按<strong>题型</strong>（单选/多选/判断/填空）、<strong>正确率</strong>（高于或低于指定百分比）、以及<strong>错题集</strong>筛选。</li>
        </ul>

        <a-typography-title :level="5">5.5 批量操作</a-typography-title>
        <ul>
          <li>勾选多个题目后，可批量设置题型，或批量删除。</li>
          <li>点击"清空题库"可一键删除当前题库的所有题目（不可恢复）。</li>
        </ul>

        <a-typography-title :level="4">六、错题管理</a-typography-title>
        <p>在错题练习的选择题库页面上，点击题库右侧的「管理错题」按钮，进入错题管理页面。</p>
        <ul>
          <li>列出当前题库中所有曾被标记为错误的题目（含手动标记的错题）。</li>
          <li>可用搜索框按题干或选项过滤。</li>
          <li>每道题显示题型、题干、选项、正确答案和解析。</li>
          <li>点击「移出错题集」可将该题从错题集中移除（删除对应的错误练习记录），该题将不再出现在错题练习中。</li>
          <li>移出后列表自动更新，无需刷新页面。</li>
        </ul>

        <a-typography-title :level="4">七、练习记忆</a-typography-title>
        <p>在「练习记忆」页面可以查看全局统计（累计练习次数、正确数、错误数、正确率），以及每个题库的详细练习记录。</p>
        <ul>
          <li>点击题库卡片可展开查看该题库的所有练习记录（按时间倒序排列）。</li>
          <li>每条记录显示：题型标签、题干、你的答案、正确答案、练习模式、作答时间。</li>
          <li>点击"清除记忆"可删除某个题库的所有练习记录并重置题目统计（不可恢复）。</li>
        </ul>

        <a-typography-title :level="4">八、数据管理</a-typography-title>
        <p>在「数据管理」页面可查看当前数据库路径、大小、题库/题目/练习记录数量，并做整库备份与恢复。</p>
        <ul>
          <li><strong>导出备份</strong>：将当前数据库另存为 .db 文件。</li>
          <li><strong>导入 · 替换</strong>：用所选 .db 完全覆盖当前库。替换前会自动备份现有数据库。</li>
          <li><strong>导入 · 合并</strong>：把所选文件的数据并入当前库，保留现有数据。同名题库可选「自动重命名导入」或「合并进已有题库」；合并时重复题目（按题干匹配）可选跳过、覆盖或全部追加。</li>
        </ul>

        <a-typography-title :level="4">九、设置</a-typography-title>
        <p>侧栏底部进入「设置」。其中三项开关是练习 / 模拟考试的<strong>默认值</strong>，进入后仍可在当场「更多」菜单里临时改，不会写回设置页。</p>
        <ul>
          <li><strong>不定项</strong>：选择题不区分单选/多选。</li>
          <li><strong>打乱答案顺序</strong>：多选答案不区分顺序。</li>
          <li><strong>打乱选项顺序</strong>：选项随机排列。</li>
        </ul>

        <a-typography-title :level="4">十、选项颜色说明</a-typography-title>
        <ul>
          <li><span style="color: #52c41a; font-weight: bold">绿色</span> — 正确答案（含单选 / 判断选错后标出的正确项）</li>
          <li><span style="color: #f5222d; font-weight: bold">红色</span> — 你选择了错误选项</li>
          <li><span style="color: #fa8c16; font-weight: bold">橙色</span> — 多选题漏选的正确项</li>
        </ul>

        <a-typography-title :level="4">十一、数据存储</a-typography-title>
        <p>
          所有题库、题目和练习记录均存储在本地 SQLite 数据库中。
          <strong>Windows</strong>：数据库文件位于 <code>SelfTest.exe</code> 所在目录下的 <code>quiz_app.db</code>，拷贝整个文件夹即可迁移数据，实现便携使用。
          <strong>Linux</strong>：位于 <code>~/.local/share/com.oneline.self-test/quiz_app.db</code>。
          数据不会上传到任何服务器，完全离线可用，无需网络连接。
        </p>

        <a-typography-title :level="4">十二、快捷键</a-typography-title>
        <ul>
          <li><kbd>Ctrl</kbd>+<kbd>F</kbd> — 在本页查找文字</li>
          <li><kbd>Enter</kbd> / <kbd>Shift</kbd>+<kbd>Enter</kbd> — 查找下一个 / 上一个</li>
          <li><kbd>Esc</kbd> — 关闭查找</li>
          <li><kbd>←</kbd> <kbd>↑</kbd> — 上一题（练习模式）</li>
          <li><kbd>→</kbd> <kbd>↓</kbd> — 下一题（练习模式）</li>
          <li><kbd>A</kbd> <kbd>B</kbd> <kbd>C</kbd> <kbd>D</kbd> — 快速选择对应选项（练习 & 考试模式）</li>
          <li><kbd>Enter</kbd> — 确认提交多选 / 填空答案</li>
        </ul>
      </a-typography>
    </div>
  </div>
</template>

<style scoped>
.guide-page {
  max-width: 800px;
  margin: 0 auto;
  line-height: 1.8;
  position: relative;
}

.guide-find-trigger {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
}

.guide-find-hint {
  color: #999;
  font-size: 12px;
}

.guide-find-bar {
  position: sticky;
  top: 0;
  z-index: 10;
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  padding: 8px 12px;
  margin-bottom: 16px;
  background: #fff;
  border: 1px solid #d9d9d9;
  border-radius: 8px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.06);
}

.guide-find-icon {
  color: #1890ff;
}

.guide-find-count {
  min-width: 72px;
  color: #666;
  font-size: 13px;
  white-space: nowrap;
}

.guide-content :deep(mark.guide-find-hit) {
  background: #ffe58f;
  color: inherit;
  padding: 0 1px;
  border-radius: 2px;
}

.guide-content :deep(mark.guide-find-hit--active) {
  background: #fa8c16;
  color: #fff;
}

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
