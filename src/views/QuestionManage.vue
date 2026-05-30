<script setup lang="ts">
// ============================================================
// 刷题助手 — 试题管理（含 Excel 导入导出）
// ============================================================
import { onMounted, ref, reactive, h } from 'vue'
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
} from '@ant-design/icons-vue'
import { invoke } from '@tauri-apps/api/tauri'
import type { Question } from '../types'

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
  editingQuestion.value = { ...q, options: [...q.options] }
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

  try {
    if ((q as any).id) {
      await invoke('update_question', {
        id: (q as any).id,
        stem: q.stem,
        type: q.type || '',
        options: q.options,
        answer: typeof q.answer === 'string' ? q.answer : JSON.parse(JSON.stringify(q.answer)),
        explanation: q.explanation || '',
      })
      message.success('更新成功')
    } else {
      await invoke('add_question', {
        bankId,
        stem: q.stem,
        type: q.type || '',
        options: q.options,
        answer: typeof q.answer === 'string' ? q.answer : JSON.parse(JSON.stringify(q.answer)),
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

// #region agent log
function agentLog(location: string, message: string, hypothesisId: string, data: Record<string, unknown> = {}) {
  fetch('http://127.0.0.1:7299/ingest/6b505492-1aff-4287-9ac5-b363715194e7', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', 'X-Debug-Session-Id': 'c82dbd' },
    body: JSON.stringify({
      sessionId: 'c82dbd',
      location,
      message,
      hypothesisId,
      data,
      timestamp: Date.now(),
      runId: 'import-excel-kylin',
    }),
  }).catch(() => {})
}
// #endregion

// ============== 导入 ==============
async function pickExcelFile() {
  if (importLoading.value) return
  importLoading.value = true
  agentLog('QuestionManage.vue:pickExcelFile', 'click import excel', 'J')
  try {
    agentLog('QuestionManage.vue:pickExcelFile', 'invoking pick_file', 'J')
    const path = await invoke<string | null>('pick_file', {
      filters: ['xlsx', 'xls', 'xlsb', 'ods', 'et'],
    })
    agentLog('QuestionManage.vue:pickExcelFile', 'pick_file returned', 'J', { hasPath: !!path })
    if (path) {
      excelPath.value = path
      importVisible.value = true
      step.value = 1
      // 获取工作表列表
      const sheets = await invoke<string[]>('get_sheet_names', { filePath: path })
      sheetNames.value = sheets
      sheetName.value = sheets[0] || ''
    }
  } catch (e) {
    agentLog('QuestionManage.vue:pickExcelFile', 'pick_file failed', 'J', { error: String(e) })
    message.error('选择文件失败: ' + e)
  } finally {
    importLoading.value = false
  }
}

async function previewExcel() {
  agentLog('QuestionManage.vue:previewExcel', 'invoking preview_excel', 'K', { path: excelPath.value })
  try {
    excelPreview.value = await invoke('preview_excel', { filePath: excelPath.value, sheetName: sheetName.value })
    agentLog('QuestionManage.vue:previewExcel', 'preview_excel returned', 'K', {
      headers: excelPreview.value?.headers?.length ?? 0,
    })
    autoDetectMapping()
  } catch (e) {
    agentLog('QuestionManage.vue:previewExcel', 'preview_excel failed', 'K', { error: String(e) })
    message.error('预览文件失败: ' + e)
    importVisible.value = false
  }
}

function autoDetectMapping() {
  if (!excelPreview.value?.headers) return
  const headers: string[] = excelPreview.value.headers.map((h: string) => h.toLowerCase())
  headers.forEach((h: string, i: number) => {
    if (/题干|题目|stem|question/.test(h)) columnMapping.stem_col = i + 1
    if (/题型|类型|type/.test(h)) columnMapping.type_col = i + 1
    if (/^a$|选项a|option.?a/.test(h)) { columnMapping.option_start_col = i + 1 }
    if (/答案|正确|answer/.test(h)) columnMapping.answer_col = i + 1
    if (/解析|解释|explanation/.test(h)) columnMapping.explanation_col = i + 1
  })
  // 统计选项列数
  let count = 0
  const startIdx = columnMapping.option_start_col - 1
  for (let i = startIdx; i < headers.length; i++) {
    if (/^[a-d]$|选项[a-d]|option.?[a-d]/.test(headers[i])) count++
    else break
  }
  if (count > 0) columnMapping.option_count = count
}

async function doImport() {
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
      answerCol: columnMapping.answer_col - 1,
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
    const path = await invoke<string | null>('save_file', { defaultName: `${bankName.value}.xlsx` })
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

    <a-table
      :row-selection="rowSelection"
      :columns="columns"
      :data-source="questions"
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
          <span v-if="record.times_attempted > 0">
            {{ Math.round(record.times_correct / record.times_attempted * 100) }}%
            ({{ record.times_correct }}/{{ record.times_attempted }})
          </span>
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
