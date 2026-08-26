<script setup lang="ts">
// ============================================================
// 理论训练考核系统 — 试题管理（含 Excel 导入导出）
// ============================================================
import { onMounted, ref, reactive, computed, h } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { message, Modal } from 'ant-design-vue'
import {
  PlusOutlined,
  EditOutlined,
  DeleteOutlined,
  ImportOutlined,
  ExportOutlined,
  ExclamationCircleOutlined,
  LeftOutlined,
  UploadOutlined,
  TagsOutlined,
  ClearOutlined,
  SearchOutlined,
} from '@ant-design/icons-vue'
import { invoke } from '@tauri-apps/api/tauri'
import { open, save } from '@tauri-apps/api/dialog'
import type { Question } from '../types'
import { questionTextMatches } from '../types'

const route = useRoute()
const router = useRouter()
const bankId = route.params.bankId as string

const questions = ref<Question[]>([])
const loading = ref(false)
const editVisible = ref(false)
const importVisible = ref(false)
const editingQuestion = ref<Partial<Question> | null>(null)

// 导入相关
const excelPath = ref('')
const sheetNames = ref<string[]>([])
const sheetName = ref('')
const excelPreview = ref<any>(null)
const step = ref(0) // 0: select, 1: mapping, 2: result
const columnMapping = reactive({
  stem_col: 1,
  type_col: null as number | null,
  option_start_col: 3,
  option_count: 4,
  option_cols: [] as number[],
  answer_col: 7,
  explanation_col: null as number | null,
})
const importResult = ref<any>(null)

// 批量设置题型
const batchTypeVisible = ref(false)
const batchTypeValue = ref('single')
const selectedRowKeys = ref<string[]>([])

// 导入时统一题型
const uniformTypeEnabled = ref(false)
const uniformTypeValue = ref('single')

// 导入重复处理策略
const duplicateStrategy = ref<string>('overwrite') // 'overwrite' | 'skip' | 'append'

const bankName = ref('')

// ===== 筛选状态 =====
const filterType = ref<string>('')
const filterAccuracyMode = ref<string>('')   // ''=不限, 'above'=高于, 'below'=低于
const filterAccuracyValue = ref<number>(50)
const filterWrongOnly = ref(false)
const questionKeyword = ref('')

/** 筛选后的题目列表 */
const filteredQuestions = computed(() => {
  let list = questions.value
  // 按题型筛选
  if (filterType.value) {
    list = list.filter(q => q.type === filterType.value)
  }
  // 按正确率筛选
  if (filterAccuracyMode.value) {
    list = list.filter(q => {
      if (q.times_attempted === 0) return false // 未练过的不计入
      const rate = q.times_correct / q.times_attempted * 100
      return filterAccuracyMode.value === 'above'
        ? rate >= filterAccuracyValue.value
        : rate < filterAccuracyValue.value
    })
  }
  // 按错题集筛选
  if (filterWrongOnly.value) {
    list = list.filter(q => q.is_wrong)
  }
  // 题干 / 选项关键字
  if (questionKeyword.value.trim()) {
    list = list.filter(q => questionTextMatches(q, questionKeyword.value))
  }
  return list
})

onMounted(async () => {
  await loadBank()
  await loadQuestions()
})

async function loadBank() {
  try {
    const bank = await invoke<any>('get_bank', { id: bankId })
    bankName.value = bank?.name || ''
  } catch (_) {}
}

async function loadQuestions() {
  loading.value = true
  try {
    questions.value = await invoke<Question[]>('list_questions', { bankId })
  } catch (e) {
    message.error('加载题目失败: ' + e)
  } finally {
    loading.value = false
  }
}

function showAdd() {
  editingQuestion.value = {
    bank_id: bankId,
    stem: '',
    type: '',
    options: ['', '', '', ''],
    answer: 'A',
    explanation: '',
  }
  editVisible.value = true
}

function showEdit(q: Question) {
  editingQuestion.value = {
    ...q,
    options: [...q.options],
    answer: Array.isArray(q.answer) ? q.answer.join(',') : q.answer,
  }
  editVisible.value = true
}

async function handleSave() {
  const q = editingQuestion.value
  if (!q?.stem?.trim()) {
    message.warning('请输入题干')
    return
  }

  // 过滤空选项
  q.options = (q.options || []).filter(o => o.trim())
  if (q.options.length === 0 && q.type !== 'fill') {
    message.warning('请至少填写一个选项')
    return
  }

  /** 保存前处理答案：多选逗号分隔字符串转为数组 */
function toSaveAnswer(answer: unknown): string | string[] {
  // 已经是数组，深拷贝一份防引用污染
  if (Array.isArray(answer)) return [...answer]
  if (typeof answer !== 'string') return ''
  const letters = answer.replace(/,/g, '').replace(/，/g, '').trim().toUpperCase()
  if (letters.length > 1 && /^[A-J]+$/.test(letters)) {
    return letters.split('')
  }
  return answer
}

try {
    if ((q as any).id) {
      await invoke('update_question', {
        id: (q as any).id,
        stem: q.stem,
        type: q.type || '',
        options: q.options,
        answer: toSaveAnswer(q.answer || ''),
        explanation: q.explanation || '',
      })
      message.success('更新成功')
    } else {
      await invoke('add_question', {
        bankId,
        stem: q.stem,
        type: q.type || '',
        options: q.options,
        answer: toSaveAnswer(q.answer || ''),
        explanation: q.explanation || '',
      })
      message.success('添加成功')
    }
    editVisible.value = false
    await loadQuestions()
  } catch (e) {
    message.error('保存失败: ' + e)
  }
}

async function handleDelete(q: Question) {
  Modal.confirm({
    title: '删除题目',
    icon: h(ExclamationCircleOutlined),
    content: '确定要删除这道题目吗？',
    okText: '删除',
    okType: 'danger',
    cancelText: '取消',
    async onOk() {
      try {
        await invoke('delete_question', { id: q.id })
        message.success('删除成功')
        await loadQuestions()
      } catch (e) {
        message.error('删除失败: ' + e)
      }
    },
  })
}

const importLoading = ref(false)

// ============== 导入 ==============
async function pickExcelFile() {
  if (importLoading.value) return
  importLoading.value = true
  try {
    const path = await open({
      title: '选择表格文件',
    }) as string | null
    if (path) {
      excelPath.value = path
      importVisible.value = true
      step.value = 1
      // 获取工作表列表
      const sheets = await invoke<string[]>('get_sheet_names', { filePath: path })
      sheetNames.value = sheets
      sheetName.value = sheets[0] || ''
      // 单选工作表自动预览
      if (sheetName.value) {
        await previewExcel()
      }
    }
  } catch (e) {
    message.error('选择文件失败: ' + e)
  } finally {
    importLoading.value = false
  }
}

async function previewExcel() {
  try {
    excelPreview.value = await invoke('preview_excel', { filePath: excelPath.value, sheetName: sheetName.value })
    autoDetectMapping()
  } catch (e) {
    message.error('预览文件失败: ' + e)
    importVisible.value = false
  }
}

/** 判断表头是否为选项列 */
function isOptionHeader(header: string): boolean {
  const raw = header.trim()
  if (!raw) return false
  const h = raw.toLowerCase()
  return (
    /^[a-j]$/.test(h) ||
    /^选项\s*[a-j0-9一二三四五六七八九十]$/.test(h) ||
    /^option\s*[a-j0-9]$/.test(h) ||
    /^[a-j]\s*选项$/.test(h) ||
    /^[a-j]选项$/.test(h) ||
    /^choice\s*[a-j0-9]$/.test(h) ||
    /^备选项?\s*[a-j0-9]$/.test(h)
  )
}

/** 从选项表头提取选项标识（用于去重合并单元格产生的重复表头） */
function getOptionLabel(header: string): string | null {
  const raw = header.trim()
  if (!raw) return null
  const h = raw.toLowerCase()
  const patterns = [
    /^([a-j])$/,
    /^选项\s*([a-j])$/,
    /^([a-j])\s*选项$/,
    /^([a-j])选项$/,
    /^option\s*([a-j])$/,
    /^choice\s*([a-j])$/,
    /^备选项?\s*([a-j])$/,
    /^选项\s*([1-9])$/,
  ]
  for (const p of patterns) {
    const m = h.match(p)
    if (m) return m[1]
  }
  return null
}

function isStemHeader(header: string): boolean {
  const h = header.trim().toLowerCase()
  return /^题干$|^题目$|^试题$|^问题$|^stem$|^question$/i.test(h)
    || (/题干|题目/.test(h) && !/题型|类型/.test(h))
}

function isTypeHeader(header: string): boolean {
  const h = header.trim().toLowerCase()
  return /^题型$|^类型$|^试题类型$|^type$/i.test(h)
}

function isAnswerHeader(header: string): boolean {
  const h = header.trim().toLowerCase()
  if (/正确率|准确率|得分率|正确次数/.test(h)) return false
  return /^答案$|^正确答案$|^answer$/i.test(h)
}

function isExplanationHeader(header: string): boolean {
  const h = header.trim().toLowerCase()
  return /^解析$|^解释$|^explanation$|^点评$/i.test(h)
    || (/解析|解释/.test(h) && !/答案/.test(h))
}

/** 按表头识别选项列（0-based），合并重复表头只保留首次出现 */
function findOptionColumnIndices(headers: string[]): number[] {
  const indices: number[] = []
  const seenLabels = new Set<string>()
  headers.forEach((h, i) => {
    if (!isOptionHeader(h)) return
    const label = getOptionLabel(h) ?? `__col_${i}`
    if (seenLabels.has(label)) return
    seenLabels.add(label)
    indices.push(i)
  })
  return indices
}

const optionDetectHint = ref('')

function autoDetectMapping() {
  if (!excelPreview.value?.headers) return
  const rawHeaders: string[] = excelPreview.value.headers
  const headers = rawHeaders.map((h: string) => h.trim().toLowerCase())
  const colCount = headers.length

  const detected = {
    stem_col: 1,
    type_col: null as number | null,
    answer_col: colCount > 0 ? colCount : 7,
    explanation_col: null as number | null,
  }

  rawHeaders.forEach((h: string, i: number) => {
    if (isStemHeader(h)) detected.stem_col = i + 1
    if (isTypeHeader(h)) detected.type_col = i + 1
    if (isAnswerHeader(h)) detected.answer_col = i + 1
    if (isExplanationHeader(h)) detected.explanation_col = i + 1
  })

  columnMapping.stem_col = detected.stem_col
  columnMapping.type_col = detected.type_col
  columnMapping.answer_col = detected.answer_col
  columnMapping.explanation_col = detected.explanation_col

  const optionColIndices = findOptionColumnIndices(rawHeaders)
  let detectSource = ''

  if (optionColIndices.length > 0) {
    columnMapping.option_cols = optionColIndices
    columnMapping.option_start_col = optionColIndices[0] + 1
    columnMapping.option_count = Math.min(optionColIndices.length, 10)
    detectSource = '表头'
    const colLabels = optionColIndices
      .slice(0, columnMapping.option_count)
      .map(i => i + 1)
      .join('、')
    optionDetectHint.value = `已自动识别选项列：第 ${colLabels} 列（共 ${columnMapping.option_count} 列，依据${detectSource}）`
  } else {
    columnMapping.option_cols = []
    const leftBound = Math.max(detected.stem_col, detected.type_col ?? 0)
    const rightCandidates = [
      detected.answer_col,
      detected.explanation_col,
      colCount + 1,
    ].filter((c): c is number => c != null && c > leftBound)
    const rightBound = Math.min(...rightCandidates) - 1

    if (rightBound > leftBound) {
      const start0 = leftBound
      const count = rightBound - leftBound
      columnMapping.option_start_col = start0 + 1
      columnMapping.option_count = Math.min(count, 10)
      columnMapping.option_cols = Array.from(
        { length: columnMapping.option_count },
        (_, i) => start0 + i,
      )
      detectSource = '列区间'
      const endCol = start0 + columnMapping.option_count
      optionDetectHint.value = `已自动识别选项列：第 ${start0 + 1}–${endCol} 列（共 ${columnMapping.option_count} 列，依据${detectSource}）`
    } else {
      optionDetectHint.value = '未能自动识别选项列，请手动设置选项起始列和列数'
    }
  }
}

async function doImport() {
  const answerCol0 = columnMapping.answer_col - 1
  const optionCols0 = columnMapping.option_cols.length > 0
    ? columnMapping.option_cols.slice(0, columnMapping.option_count)
    : Array.from(
        { length: columnMapping.option_count },
        (_, i) => columnMapping.option_start_col - 1 + i,
      )

  if (optionCols0.some(col => col === answerCol0)) {
    message.error('列映射有误：选项列与答案列重叠，请检查自动识别结果或手动调整')
    return
  }
  if (optionCols0.some(col => col === columnMapping.stem_col - 1)) {
    message.error('列映射有误：选项列与题干列重叠，请检查自动识别结果或手动调整')
    return
  }

  try {
    const forceType: string | null = uniformTypeEnabled.value ? uniformTypeValue.value : null
    const strategy: string = duplicateStrategy.value

    importResult.value = await invoke('import_questions', {
      bankId,
      filePath: excelPath.value,
      sheetName: sheetName.value,
      stemCol: columnMapping.stem_col - 1,
      typeCol: columnMapping.type_col != null ? columnMapping.type_col - 1 : null,
      optionStartCol: columnMapping.option_start_col - 1,
      optionCount: columnMapping.option_count,
      optionCols: optionCols0,
      answerCol: answerCol0,
      explanationCol: columnMapping.explanation_col != null ? columnMapping.explanation_col - 1 : null,
      forceType,
      duplicateStrategy: strategy,
    })
    step.value = 2
    await loadQuestions()
    const parts: string[] = [`成功 ${importResult.value.success} 题`]
    if (importResult.value.skipped > 0) parts.push(`跳过 ${importResult.value.skipped} 题`)
    if (importResult.value.overwritten > 0) parts.push(`覆盖 ${importResult.value.overwritten} 题`)
    if (importResult.value.failed > 0) parts.push(`失败 ${importResult.value.failed} 题`)
    message.success(`导入完成：${parts.join('，')}`)
  } catch (e) {
    message.error('导入失败: ' + e)
  }
}

// ============== 批量设置题型 ==============
async function handleBatchSetType() {
  if (selectedRowKeys.value.length === 0) {
    message.warning('请先选择题号')
    return
  }
  try {
    const count = await invoke<number>('batch_set_type', {
      questionIds: selectedRowKeys.value,
      qType: batchTypeValue.value,
    })
    message.success(`已成功将 ${count} 道题设为 ${getTypeLabel(batchTypeValue.value)}`)
    batchTypeVisible.value = false
    selectedRowKeys.value = []
    await loadQuestions()
  } catch (e) {
    message.error('批量设置失败: ' + e)
  }
}

function getTypeLabel(type: string): string {
  const map: Record<string, string> = {
    single: '单选题',
    multiple: '多选题',
    judge: '判断题',
    fill: '填空题',
  }
  return map[type] || type
}

// ============== 批量删除 ==============
async function handleBatchDelete() {
  if (selectedRowKeys.value.length === 0) {
    message.warning('请先选择题号')
    return
  }
  Modal.confirm({
    title: '批量删除',
    icon: h(ExclamationCircleOutlined),
    content: `确定要删除选中的 ${selectedRowKeys.value.length} 道题目吗？此操作不可恢复。`,
    okText: '删除',
    okType: 'danger',
    async onOk() {
      try {
        const count = await invoke<number>('batch_delete_questions', {
          questionIds: selectedRowKeys.value,
        })
        message.success(`成功删除 ${count} 道题`)
        selectedRowKeys.value = []
        await loadQuestions()
      } catch (e) {
        message.error('批量删除失败: ' + e)
      }
    },
  })
}

async function handleClearBank() {
  Modal.confirm({
    title: '清空题库',
    icon: h(ExclamationCircleOutlined),
    content: `确定要清空题库「${bankName.value}」中的所有题目吗？此操作不可恢复。`,
    okText: '清空',
    okType: 'danger',
    async onOk() {
      try {
        const count = await invoke<number>('clear_bank_questions', {
          bankId,
        })
        message.success(`已清空 ${count} 道题`)
        selectedRowKeys.value = []
        await loadQuestions()
      } catch (e) {
        message.error('清空失败: ' + e)
      }
    },
  })
}

// ============== 导出 ==============
async function doExport() {
  try {
    const path = await save({
      title: '保存为',
      defaultPath: `${bankName.value}.xlsx`,
    })
    if (!path) return
    await invoke('export_questions', { bankId, savePath: path })
    message.success(`导出成功：${path}`)
  } catch (e) {
    message.error('导出失败: ' + e)
  }
}

const rowSelection = {
  selectedRowKeys: selectedRowKeys,
  onChange: (keys: string[]) => {
    selectedRowKeys.value = keys
  },
}

const columns = [
  { title: '题型', dataIndex: 'type', key: 'type', width: 100 },
  { title: '题干', dataIndex: 'stem', key: 'stem', ellipsis: true },
  { title: '答案', key: 'answer', width: 80 },
  { title: '正确率', key: 'rate', width: 100 },
  { title: '错题集', key: 'wrong', width: 80 },
  { title: '操作', key: 'action', width: 160 },
]
</script>

<template>
  <div>
    <div style="margin-bottom: 16px; display: flex; justify-content: space-between; align-items: center">
      <a-space>
        <a-button @click="router.push('/')"><LeftOutlined /> 返回</a-button>
        <a-tag color="blue">{{ bankName }}</a-tag>
      </a-space>
      <a-space>
        <a-button :loading="importLoading" @click="pickExcelFile"><ImportOutlined /> 导入 Excel</a-button>
        <a-button @click="doExport"><ExportOutlined /> 导出 Excel</a-button>
        <a-button :disabled="selectedRowKeys.length === 0" @click="batchTypeVisible = true"><TagsOutlined /> 批量题型</a-button>
        <a-button :disabled="selectedRowKeys.length === 0" danger @click="handleBatchDelete"><DeleteOutlined /> 批量删除</a-button>
        <a-button danger @click="handleClearBank"><ClearOutlined /> 清空题库</a-button>
        <a-button type="primary" @click="showAdd"><PlusOutlined /> 添加题目</a-button>
      </a-space>
    </div>

    <!-- 筛选栏 -->
    <div style="margin-bottom: 16px; padding: 12px 16px; background: #fafafa; border-radius: 8px; display: flex; align-items: center; gap: 16px; flex-wrap: wrap">
      <span style="font-size: 13px; color: #666; font-weight: 500">筛选：</span>
      <a-input
        v-model:value="questionKeyword"
        placeholder="搜索题干或选项"
        allow-clear
        style="width: 220px"
      >
        <template #prefix><SearchOutlined style="color: #bfbfbf" /></template>
      </a-input>
      <a-select
        v-model:value="filterType"
        style="width: 120px"
        placeholder="全部题型"
        allow-clear
        @clear="filterType = ''"
      >
        <a-select-option value="">全部题型</a-select-option>
        <a-select-option value="single">单选题</a-select-option>
        <a-select-option value="multiple">多选题</a-select-option>
        <a-select-option value="judge">判断题</a-select-option>
        <a-select-option value="fill">填空题</a-select-option>
      </a-select>

      <a-select v-model:value="filterAccuracyMode" style="width: 130px" placeholder="正确率" allow-clear @clear="filterAccuracyMode = ''">
        <a-select-option value="">正确率不限</a-select-option>
        <a-select-option value="above">正确率高于</a-select-option>
        <a-select-option value="below">正确率低于</a-select-option>
      </a-select>
      <a-input-number
        v-if="filterAccuracyMode"
        v-model:value="filterAccuracyValue"
        :min="0"
        :max="100"
        :formatter="(v: any) => `${v}%`"
        :parser="(v: any) => v.replace('%', '')"
        style="width: 90px"
        size="small"
      />

      <a-checkbox v-model:checked="filterWrongOnly" style="margin-left: 4px">
        只显示错题集题目
      </a-checkbox>

      <span style="color: #999; font-size: 12px; margin-left: auto">
        共 <strong>{{ filteredQuestions.length }}</strong> 题
        <template v-if="filteredQuestions.length !== questions.length">
          （筛选自 {{ questions.length }} 题）
        </template>
      </span>
    </div>

    <a-table
      :row-selection="rowSelection"
      :columns="columns"
      :data-source="filteredQuestions"
      :loading="loading"
      row-key="id"
      :pagination="{ defaultPageSize: 20, pageSizeOptions: ['10', '20', '50', '100', '200'], showSizeChanger: true, showQuickJumper: true, showTotal: (total: number, range: number[]) => `第 ${range[0]}-${range[1]} 题 / 共 ${total} 题` }"
    >
      <template #bodyCell="{ column, record }">
        <template v-if="column.key === 'type'">
          <span>{{ getTypeLabel(record.type) || '-' }}</span>
        </template>
        <template v-if="column.key === 'answer'">
          <a-tag color="green">
            {{ Array.isArray(record.answer) ? (record.answer as string[]).join(', ') : record.answer }}
          </a-tag>
        </template>
        <template v-if="column.key === 'rate'">
          <span v-if="record.times_attempted > 0" :style="{ color: record.times_correct / record.times_attempted >= 0.8 ? '#52c41a' : '#f5222d' }">
            {{ Math.round(record.times_correct / record.times_attempted * 100) }}%
            ({{ record.times_correct }}/{{ record.times_attempted }})
          </span>
          <span v-else style="color: #ccc">-</span>
        </template>
        <template v-if="column.key === 'wrong'">
          <a-tag v-if="record.is_wrong" color="red" style="font-size: 11px">错题</a-tag>
          <span v-else style="color: #ccc">-</span>
        </template>
        <template v-if="column.key === 'action'">
          <a-space>
            <a-button size="small" @click="showEdit(record)"><EditOutlined /></a-button>
            <a-button size="small" danger @click="handleDelete(record)"><DeleteOutlined /></a-button>
          </a-space>
        </template>
      </template>
    </a-table>

    <!-- 编辑/添加对话框 -->
    <!-- 批量设置题型对话框 -->
    <a-modal
      v-model:open="batchTypeVisible"
      title="批量设置题型"
      width="400px"
      @ok="handleBatchSetType"
    >
      <div style="margin-bottom: 16px">
        已选中 <strong>{{ selectedRowKeys.length }}</strong> 道题，将统一设为：
      </div>
      <a-select v-model:value="batchTypeValue" style="width: 100%">
        <a-select-option value="single">单选题</a-select-option>
        <a-select-option value="multiple">多选题</a-select-option>
        <a-select-option value="judge">判断题</a-select-option>
        <a-select-option value="fill">填空题</a-select-option>
      </a-select>
    </a-modal>

    <!-- 编辑/添加对话框 -->
    <a-modal
      v-model:open="editVisible"
      :title="(editingQuestion as any)?.id ? '编辑题目' : '添加题目'"
      width="720px"
      @ok="handleSave"
    >
      <a-form layout="vertical" v-if="editingQuestion">
        <a-form-item label="题型">
          <a-select v-model:value="editingQuestion.type" placeholder="选择题型（可选）" allow-clear>
            <a-select-option value="single">单选题</a-select-option>
            <a-select-option value="multiple">多选题</a-select-option>
            <a-select-option value="judge">判断题</a-select-option>
            <a-select-option value="fill">填空题</a-select-option>
          </a-select>
        </a-form-item>
        <a-form-item label="题干">
          <a-textarea v-model:value="editingQuestion.stem" :rows="3" placeholder="请输入题干" />
        </a-form-item>
        <a-form-item label="选项">
          <a-space direction="vertical" style="width: 100%">
            <a-input
              v-for="(_, idx) in editingQuestion.options"
              :key="idx"
              :value="editingQuestion?.options?.[idx] || ''"
              :addon-before="String.fromCharCode(65 + idx)"
              :placeholder="`选项 ${String.fromCharCode(65 + idx)}`"
              @input="(e: Event) => { const t = (e.target as HTMLInputElement).value; if (editingQuestion?.options) editingQuestion.options[idx] = t }"
            />
          </a-space>
        </a-form-item>
        <a-form-item label="正确答案">
          <a-input v-model:value="editingQuestion.answer" placeholder="A 或 A,C（多选逗号分隔）" />
          <div style="color: #999; font-size: 12px; margin-top: 4px">
            单选题填字母如 A，多选题逗号分隔如 A,C，判断题填 A 或 B，填空题直接填文本
          </div>
        </a-form-item>
        <a-form-item label="解析（可选）">
          <a-textarea v-model:value="editingQuestion.explanation" :rows="2" placeholder="题目解析" />
        </a-form-item>
      </a-form>
    </a-modal>

    <!-- 导入对话框 -->
    <a-modal
      v-model:open="importVisible"
      title="导入题目"
      width="800px"
      :footer="null"
    >
      <!-- 工作表选择（多工作表文件） -->
      <div v-if="sheetNames.length > 1" style="margin-bottom: 16px">
        <a-space>
          <span>工作表：</span>
          <a-select
            v-model:value="sheetName"
            style="width: 300px"
            @change="step === 1 && previewExcel()"
          >
            <a-select-option
              v-for="s in sheetNames"
              :key="s"
              :value="s"
            >
              {{ s }}
            </a-select-option>
          </a-select>
          <a-button size="small" @click="previewExcel()">重新加载</a-button>
        </a-space>
      </div>

      <!-- Step 1: 列映射 -->
      <div v-if="step === 1 && excelPreview">
        <a-alert message="已加载文件，请确认列映射" type="info" show-icon style="margin-bottom: 16px" />

        <a-descriptions :column="2" size="small" bordered style="margin-bottom: 16px">
          <a-descriptions-item label="标题行">
            {{ excelPreview.headers.join('  |  ') }}
          </a-descriptions-item>
          <a-descriptions-item label="数据行数">{{ excelPreview.total_rows }}</a-descriptions-item>
        </a-descriptions>

        <a-form layout="inline" style="margin-bottom: 16px">
          <a-form-item label="题干列">
            <a-input-number v-model:value="columnMapping.stem_col" :min="1" style="width: 70px" />
          </a-form-item>
          <a-form-item label="题型列">
            <a-input-number v-model:value="columnMapping.type_col" :min="1" style="width: 70px" />
            <span style="color: #999; font-size: 12px">无则留空</span>
          </a-form-item>
          <a-form-item label="选项起始列">
            <a-input-number v-model:value="columnMapping.option_start_col" :min="1" style="width: 70px" />
          </a-form-item>
          <a-form-item label="选项列数">
            <a-input-number v-model:value="columnMapping.option_count" :min="1" :max="10" style="width: 70px" />
          </a-form-item>
          <a-form-item label="答案列">
            <a-input-number v-model:value="columnMapping.answer_col" :min="1" style="width: 70px" />
          </a-form-item>
          <a-form-item label="解析列">
            <a-input-number v-model:value="columnMapping.explanation_col" :min="1" style="width: 70px" />
          </a-form-item>
        </a-form>
        <div v-if="optionDetectHint" style="margin: -8px 0 16px; color: #1890ff; font-size: 12px">
          {{ optionDetectHint }}
          <a-button type="link" size="small" style="padding: 0 4px" @click="autoDetectMapping">重新识别</a-button>
        </div>

        <!-- 统一设置题型 -->
        <div style="margin-bottom: 16px; padding: 12px; background: #fafafa; border-radius: 6px">
          <a-checkbox v-model:checked="uniformTypeEnabled">
            统一设置所有导入题目的题型
          </a-checkbox>
          <a-select
            v-if="uniformTypeEnabled"
            v-model:value="uniformTypeValue"
            style="width: 160px; margin-left: 12px"
            :disabled="!uniformTypeEnabled"
          >
            <a-select-option value="single">单选题</a-select-option>
            <a-select-option value="multiple">多选题</a-select-option>
            <a-select-option value="judge">判断题</a-select-option>
            <a-select-option value="fill">填空题</a-select-option>
          </a-select>
        </div>

        <!-- 重复题目处理方式 -->
        <div style="margin-bottom: 16px; padding: 12px; background: #fff7e6; border-radius: 6px; border: 1px solid #ffd591">
          <div style="margin-bottom: 8px; font-weight: bold">重复题目处理方式</div>
          <a-radio-group v-model:value="duplicateStrategy" button-style="solid">
            <a-radio-button value="overwrite">覆盖已有</a-radio-button>
            <a-radio-button value="skip">跳过重复</a-radio-button>
            <a-radio-button value="append">全部追加</a-radio-button>
          </a-radio-group>
          <div style="margin-top: 6px; color: #999; font-size: 12px">
            以题干匹配判断是否重复，覆盖会更新已有题目的选项、答案和解析
          </div>
        </div>

        <!-- 预览 -->
        <div style="margin-bottom: 16px">
          <div style="font-weight: bold; margin-bottom: 8px">数据预览（前 3 行）：</div>
          <a-table
            :columns="excelPreview.headers.map((h: string, i: number) => ({ title: h, dataIndex: String(i), key: i }))"
            :data-source="excelPreview.preview.map((row: string[], ri: number) => {
              const obj: any = { key: ri }
              row.forEach((cell, ci) => obj[ci] = cell)
              return obj
            })"
            :pagination="false"
            size="small"
          />
        </div>

        <div style="text-align: right">
          <a-space>
            <a-button @click="importVisible = false">取消</a-button>
            <a-button type="primary" @click="doImport()"><UploadOutlined /> 开始导入</a-button>
          </a-space>
        </div>
      </div>

      <!-- 重复检测确认对话框（已合并到列映射区域） -->

      <!-- Step 2: 结果 -->
      <div v-if="step === 2 && importResult">
        <a-result
          :status="importResult.failed === 0 ? 'success' : 'warning'"
          :title="importResult.failed === 0 ? '导入完成' : '导入完成（部分失败）'"
          :sub-title="`成功 ${importResult.success} 题${importResult.skipped > 0 ? '，跳过 ' + importResult.skipped + ' 题' : ''}${importResult.overwritten > 0 ? '，覆盖 ' + importResult.overwritten + ' 题' : ''}${importResult.failed > 0 ? '，失败 ' + importResult.failed + ' 题' : ''}`"
        >
          <template #extra>
            <a-button type="primary" @click="importVisible = false">关闭</a-button>
          </template>
        </a-result>
        <div v-if="importResult.errors.length > 0" style="max-height: 200px; overflow: auto; margin-top: 16px">
          <div v-for="(err, idx) in importResult.errors" :key="idx" style="color: #f5222d; font-size: 13px">
            {{ err }}
          </div>
        </div>
      </div>
    </a-modal>
  </div>
</template>
