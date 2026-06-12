<script setup lang="ts">
// ============================================================
// 刷题助手 — 数据管理（数据库导入 / 导出 / 合并）
// ============================================================
import { onMounted, ref, computed, h } from 'vue'
import { message, Modal } from 'ant-design-vue'
import {
  DatabaseOutlined,
  ExportOutlined,
  ImportOutlined,
  ReloadOutlined,
  ExclamationCircleOutlined,
} from '@ant-design/icons-vue'
import { invoke } from '@tauri-apps/api/tauri'
import { open, save } from '@tauri-apps/api/dialog'
import { useBankStore } from '../stores/bank'
import type {
  DatabaseInfo,
  SourceDatabasePreview,
  ImportReplaceResult,
  MergeResult,
  SourceBankPreview,
} from '../types'

const bankStore = useBankStore()
const loading = ref(false)
const dbInfo = ref<DatabaseInfo | null>(null)

const importVisible = ref(false)
const importStep = ref(1)
const importLoading = ref(false)
const sourcePreview = ref<SourceDatabasePreview | null>(null)
const importMode = ref<'replace' | 'merge'>('replace')
const bankConflictStrategy = ref<'rename' | 'merge_into_existing'>('rename')
const duplicateStrategy = ref<'skip' | 'overwrite' | 'append'>('skip')
const importResult = ref<ImportReplaceResult | MergeResult | null>(null)
const resultType = ref<'replace' | 'merge' | null>(null)

const bankColumns = [
  { title: '题库名称', dataIndex: 'name', key: 'name' },
  { title: '题目数', dataIndex: 'question_count', key: 'question_count', width: 90 },
  { title: '创建时间', dataIndex: 'created_at', key: 'created_at', width: 170 },
  {
    title: '状态',
    key: 'conflict',
    width: 80,
  },
]

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / 1024 / 1024).toFixed(2)} MB`
}

function shortPath(path: string): string {
  if (path.length <= 60) return path
  return '...' + path.slice(-57)
}

async function loadInfo() {
  loading.value = true
  try {
    dbInfo.value = await invoke<DatabaseInfo>('get_database_info')
  } catch (e) {
    message.error('加载数据库信息失败: ' + e)
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  loadInfo()
})

async function handleExport() {
  const ts = new Date().toISOString().slice(0, 19).replace(/[-:T]/g, '').slice(0, 8)
    + '_' + new Date().toTimeString().slice(0, 8).replace(/:/g, '')
  const defaultName = `刷题助手_备份_${ts}.db`
  const path = await save({
    title: '导出数据库备份',
    defaultPath: defaultName,
    filters: [{ name: 'SQLite 数据库', extensions: ['db'] }],
  }) as string | null
  if (!path) return
  try {
    const saved = await invoke<string>('export_database', { savePath: path })
    message.success(`导出成功：${saved}`)
  } catch (e) {
    message.error('导出失败: ' + e)
  }
}

function resetImportWizard() {
  importStep.value = 1
  sourcePreview.value = null
  importMode.value = 'replace'
  bankConflictStrategy.value = 'rename'
  duplicateStrategy.value = 'skip'
  importResult.value = null
  resultType.value = null
}

async function startImport() {
  resetImportWizard()
  const path = await open({
    title: '选择数据库文件',
    filters: [{ name: 'SQLite 数据库', extensions: ['db'] }],
  }) as string | null
  if (!path) return

  importLoading.value = true
  try {
    sourcePreview.value = await invoke<SourceDatabasePreview>('preview_source_database', {
      filePath: path,
    })
    importVisible.value = true
    importStep.value = 1
  } catch (e) {
    message.error('读取数据库失败: ' + e)
  } finally {
    importLoading.value = false
  }
}

const previewBanks = computed(() => sourcePreview.value?.banks ?? [])

function rowClassName(record: SourceBankPreview): string {
  return importMode.value === 'merge' && record.name_conflict ? 'conflict-row' : ''
}

async function executeImport() {
  if (!sourcePreview.value) return

  if (importMode.value === 'replace') {
    Modal.confirm({
      title: '确认替换数据库',
      icon: h(ExclamationCircleOutlined),
      content: '将用所选文件完全覆盖当前数据库，操作前会自动备份当前库。此操作不可撤销，确定继续吗？',
      okText: '确认替换',
      okType: 'danger',
      cancelText: '取消',
      async onOk() {
        importLoading.value = true
        try {
          const res = await invoke<ImportReplaceResult>('import_database', {
            filePath: sourcePreview.value!.file_path,
          })
          importResult.value = res
          resultType.value = 'replace'
          importStep.value = 4
          await loadInfo()
          await bankStore.fetchBanks()
          message.success('数据库已替换，当前库已自动备份')
        } catch (e) {
          message.error('导入失败: ' + e)
        } finally {
          importLoading.value = false
        }
      },
    })
    return
  }

  importLoading.value = true
  try {
    const res = await invoke<MergeResult>('merge_database', {
      filePath: sourcePreview.value.file_path,
      bankConflictStrategy: bankConflictStrategy.value,
      duplicateStrategy: duplicateStrategy.value,
    })
    importResult.value = res
    resultType.value = 'merge'
    importStep.value = 4
    await loadInfo()
    await bankStore.fetchBanks()
    message.success('数据库合并完成')
  } catch (e) {
    message.error('合并失败: ' + e)
  } finally {
    importLoading.value = false
  }
}

function closeImport() {
  importVisible.value = false
  resetImportWizard()
}
</script>

<template>
  <div class="data-manage-page">
    <div class="page-header">
      <h2 style="margin: 0">
        <DatabaseOutlined style="margin-right: 8px" />
        数据管理
      </h2>
      <a-space>
        <a-button :loading="loading" @click="loadInfo">
          <ReloadOutlined /> 刷新
        </a-button>
        <a-button :loading="importLoading" @click="startImport">
          <ImportOutlined /> 导入数据库
        </a-button>
        <a-button type="primary" @click="handleExport">
          <ExportOutlined /> 导出备份
        </a-button>
      </a-space>
    </div>

    <a-spin :spinning="loading">
      <!-- 数据库概览 -->
      <a-card title="当前数据库" style="margin-bottom: 16px">
        <template v-if="dbInfo">
          <a-descriptions :column="2" size="small" bordered>
            <a-descriptions-item label="文件路径" :span="2">
              <span :title="dbInfo.path">{{ shortPath(dbInfo.path) }}</span>
            </a-descriptions-item>
            <a-descriptions-item label="文件大小">{{ formatSize(dbInfo.size_bytes) }}</a-descriptions-item>
            <a-descriptions-item label="题库数">{{ dbInfo.bank_count }}</a-descriptions-item>
            <a-descriptions-item label="题目数">{{ dbInfo.question_count }}</a-descriptions-item>
            <a-descriptions-item label="练习记录">{{ dbInfo.record_count }}</a-descriptions-item>
          </a-descriptions>

          <div style="margin-top: 16px; font-weight: 500; margin-bottom: 8px">所含题库</div>
          <a-table
            :columns="bankColumns"
            :data-source="dbInfo.banks"
            :pagination="false"
            size="small"
            row-key="name"
          >
            <template #bodyCell="{ column }">
              <template v-if="column.key === 'conflict'">
                <span style="color: #ccc">—</span>
              </template>
            </template>
            <template #emptyText>暂无题库</template>
          </a-table>
        </template>
      </a-card>

      <a-alert
        type="info"
        show-icon
        message="导入说明"
        description="导入数据库时，先选择 .db 文件，再选择「替换」（完全覆盖）或「合并」（保留现有数据）。替换前会自动备份当前数据库。"
      />
    </a-spin>

    <!-- 导入向导 -->
    <a-modal
      v-model:open="importVisible"
      title="导入数据库"
      width="720px"
      :footer="null"
      :destroy-on-close="true"
      @cancel="closeImport"
    >
      <a-steps :current="importStep - 1" size="small" style="margin-bottom: 24px">
        <a-step title="预览" />
        <a-step title="导入方式" />
        <a-step title="确认" />
        <a-step title="完成" />
      </a-steps>

      <!-- Step 1: 预览 -->
      <div v-if="importStep === 1 && sourcePreview">
        <a-descriptions :column="2" size="small" bordered style="margin-bottom: 16px">
          <a-descriptions-item label="文件" :span="2">
            <span :title="sourcePreview.file_path">{{ shortPath(sourcePreview.file_path) }}</span>
          </a-descriptions-item>
          <a-descriptions-item label="大小">{{ formatSize(sourcePreview.size_bytes) }}</a-descriptions-item>
          <a-descriptions-item label="题库数">{{ sourcePreview.bank_count }}</a-descriptions-item>
          <a-descriptions-item label="题目数">{{ sourcePreview.question_count }}</a-descriptions-item>
          <a-descriptions-item label="练习记录">{{ sourcePreview.record_count }}</a-descriptions-item>
        </a-descriptions>

        <div style="font-weight: 500; margin-bottom: 8px">所含题库</div>
        <a-table
          :columns="bankColumns"
          :data-source="previewBanks"
          :pagination="previewBanks.length > 8 ? { pageSize: 8 } : false"
          size="small"
          row-key="name"
          :row-class-name="rowClassName"
        >
          <template #bodyCell="{ column, record }">
            <template v-if="column.key === 'conflict'">
              <a-tag v-if="record.name_conflict" color="orange">同名</a-tag>
              <span v-else style="color: #ccc">—</span>
            </template>
          </template>
        </a-table>

        <div style="text-align: right; margin-top: 16px">
          <a-button type="primary" @click="importStep = 2">下一步</a-button>
        </div>
      </div>

      <!-- Step 2: 选择方式 -->
      <div v-if="importStep === 2">
        <a-radio-group v-model:value="importMode" style="width: 100%">
          <a-space direction="vertical" style="width: 100%">
            <a-radio value="replace" style="display: flex; align-items: flex-start; padding: 12px; border: 1px solid #f0f0f0; border-radius: 8px; width: 100%">
              <div>
                <div style="font-weight: 500">替换</div>
                <div style="color: #999; font-size: 12px; margin-top: 4px">
                  用所选文件完全覆盖当前数据库（操作前自动备份当前库）
                </div>
              </div>
            </a-radio>
            <a-radio value="merge" style="display: flex; align-items: flex-start; padding: 12px; border: 1px solid #f0f0f0; border-radius: 8px; width: 100%">
              <div>
                <div style="font-weight: 500">合并</div>
                <div style="color: #999; font-size: 12px; margin-top: 4px">
                  将所选文件的数据并入当前库，保留现有数据
                </div>
              </div>
            </a-radio>
          </a-space>
        </a-radio-group>

        <div style="text-align: right; margin-top: 16px">
          <a-space>
            <a-button @click="importStep = 1">上一步</a-button>
            <a-button type="primary" @click="importStep = 3">下一步</a-button>
          </a-space>
        </div>
      </div>

      <!-- Step 3: 确认 -->
      <div v-if="importStep === 3">
        <a-alert
          v-if="importMode === 'replace'"
          type="error"
          show-icon
          message="即将替换整个数据库"
          description="当前所有题库、题目和练习记录将被覆盖。系统会在替换前自动创建备份文件。"
          style="margin-bottom: 16px"
        />

        <template v-if="importMode === 'merge'">
          <div style="margin-bottom: 12px; font-weight: 500">同名题库处理</div>
          <a-radio-group v-model:value="bankConflictStrategy" button-style="solid" style="margin-bottom: 16px">
            <a-radio-button value="rename">自动重命名导入</a-radio-button>
            <a-radio-button value="merge_into_existing">合并进已有题库</a-radio-button>
          </a-radio-group>

          <template v-if="bankConflictStrategy === 'merge_into_existing'">
            <div style="margin-bottom: 8px; font-weight: 500">题目重复处理（按题干匹配）</div>
            <a-radio-group v-model:value="duplicateStrategy" style="margin-bottom: 16px">
              <a-radio value="skip">跳过重复</a-radio>
              <a-radio value="overwrite">覆盖已有</a-radio>
              <a-radio value="append">全部追加</a-radio>
            </a-radio-group>
          </template>

          <div style="font-weight: 500; margin-bottom: 8px">源库题库预览</div>
          <a-table
            :columns="bankColumns"
            :data-source="previewBanks"
            :pagination="false"
            size="small"
            row-key="name"
            :row-class-name="rowClassName"
            style="margin-bottom: 16px"
          >
            <template #bodyCell="{ column, record }">
              <template v-if="column.key === 'conflict'">
                <a-tag v-if="record.name_conflict" color="orange">同名</a-tag>
                <span v-else style="color: #ccc">—</span>
              </template>
            </template>
          </a-table>
        </template>

        <div style="text-align: right">
          <a-space>
            <a-button @click="importStep = 2">上一步</a-button>
            <a-button
              type="primary"
              :danger="importMode === 'replace'"
              :loading="importLoading"
              @click="executeImport"
            >
              {{ importMode === 'replace' ? '确认替换' : '开始合并' }}
            </a-button>
          </a-space>
        </div>
      </div>

      <!-- Step 4: 完成 -->
      <div v-if="importStep === 4 && importResult">
        <a-result
          status="success"
          title="操作完成"
        >
          <template #subTitle>
            <template v-if="resultType === 'replace'">
              数据库已替换。备份文件：{{ (importResult as ImportReplaceResult).backup_path }}
            </template>
            <template v-else>
              新增题库 {{ (importResult as MergeResult).banks_added }} 个，
              合并题库 {{ (importResult as MergeResult).banks_merged }} 个，
              新增题目 {{ (importResult as MergeResult).questions_added }} 道
              <template v-if="(importResult as MergeResult).questions_skipped > 0">
                ，跳过 {{ (importResult as MergeResult).questions_skipped }} 道
              </template>
              <template v-if="(importResult as MergeResult).questions_overwritten > 0">
                ，覆盖 {{ (importResult as MergeResult).questions_overwritten }} 道
              </template>
            </template>
          </template>
          <template #extra>
            <a-button type="primary" @click="closeImport">关闭</a-button>
          </template>
        </a-result>
        <div
          v-if="resultType === 'merge' && (importResult as MergeResult).errors.length > 0"
          style="max-height: 120px; overflow: auto; margin-top: 8px"
        >
          <div
            v-for="(err, idx) in (importResult as MergeResult).errors"
            :key="idx"
            style="color: #f5222d; font-size: 13px"
          >
            {{ err }}
          </div>
        </div>
      </div>
    </a-modal>
  </div>
</template>

<style scoped>
.data-manage-page {
  max-width: 900px;
  margin: 0 auto;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

:deep(.conflict-row) {
  background: #fff7e6;
}
</style>
