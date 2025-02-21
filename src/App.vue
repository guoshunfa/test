<script setup lang="ts">
import { ref } from 'vue'
import {
  Headset,
  VideoPlay,
  Timer,
  Sort,
  Switch,
  Upload
} from '@element-plus/icons-vue'
// 导入 invoke 方法
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { listen } from '@tauri-apps/api/event';
import { ElMessage } from 'element-plus'

interface FormState {
  input_path: string
  audio_path: string
  is_folder: boolean
  order: 'sequence' | 'random'
  duration: number
  progress: number
}

const form = ref<FormState>({
  input_path: '',
  audio_path: '',
  is_folder: false,
  order: 'sequence',
  duration: 40,
  progress: 0
})

// 添加时间相关的响应式变量
const startTime = ref<number>(0)
const elapsedTime = ref<string>('')
const lastProcessTime = ref<string>('')

// 选择视频文件
const onAddFolder = async () => {
  try {
    // 根据是否选择单个文件还是文件夹使用不同的选择器
    const selected = await open({
      multiple: false,
      filters: [
        {
          name: 'Video',
          extensions: ['mp4', 'mov', 'avi', 'wmv', 'flv', 'mkv']
        }
      ]
    });

    if (selected) {
      // selected 现在就是完整的文件系统路径
      const fullPath = selected as string;
      console.log('选择的完整路径:', fullPath);

      // 更新表单数据
      form.value.input_path = fullPath;
      form.value.is_folder = false;
    }
  } catch (err) {
    console.error('选择文件失败:', err);
  }
};

// 新增一个选择文件夹的方法
const onAddFolderPath = async () => {
  try {
    const selected = await open({
      directory: true,
      multiple: false,
    });

    if (selected) {
      // selected 是完整的文件夹路径
      const folderPath = selected as string;
      console.log('选择的文件夹完整路径:', folderPath);

      // 更新表单数据
      form.value.input_path = folderPath;
      form.value.is_folder = true;
    }
  } catch (err) {
    console.error('选择文件夹失败:', err);
  }
};

// 选择音频文件
const onSelectAudio = async () => {
  try {
    const selected = await open({
      multiple: false,
      filters: [
        {
          name: 'Audio',
          extensions: ['mp3', 'wav', 'm4a', 'aac']
        }
      ]
    });

    if (selected) {
      const audioPath = selected as string;
      console.log('选择的音频文件完整路径:', audioPath);
      form.value.audio_path = audioPath;
    }
  } catch (err) {
    console.error('选择音频文件失败:', err);
  }
};

// 提交处理
const handleSubmit = async () => {
  try {
    if (!form.value.input_path) {
      return
    }

    // 重置进度并记录开始时间
    form.value.progress = 0
    startTime.value = Date.now()
    elapsedTime.value = ''

    // 更新计时的函数
    const updateElapsedTime = () => {
      const seconds = Math.floor((Date.now() - startTime.value) / 1000)
      const minutes = Math.floor(seconds / 60)
      const remainingSeconds = seconds % 60
      elapsedTime.value = `${minutes}分${remainingSeconds}秒`
    }

    // 启动定时器
    const timer = setInterval(updateElapsedTime, 1000)

    // 监听处理进度
    await listen<number>('process-progress', (event) => {
      form.value.progress = event.payload
      if (event.payload === 100) {
        clearInterval(timer)
        lastProcessTime.value = elapsedTime.value
        ElMessage.success(`视频剪辑完成！总用时：${elapsedTime.value}`)
      }
      console.log(`处理进度: ${event.payload}%`)
    })

    // 发送处理请求
    await invoke('start_editing', {
      inputPath: form.value.input_path, // 替换为实际路径
      audioPath: form.value.audio_path, // 替换为实际路径
      isFolder: form.value.is_folder, // 根据用户选择设置
      order: form.value.order, // 根据用户选择设置
      duration: form.value.duration, // 根据用户选择设置
    }).then((res) => {
      console.log('res', res);
    }).catch((err) => {
      console.error('剪辑失败：', err);
    });
  } catch (err) {
    console.error('处理失败:', err)
  }
}

// 清除音频
const onClearAudio = () => {
  form.value.audio_path = '';
  ElMessage.success('音频已清除');
}

</script>

<template>
  <div class="container">
    <el-card class="form-card">
      <template #header>
        <div class="card-header">
          <h2>视频剪辑工具</h2>
          <p class="subtitle">支持视频合并与音频导入</p>
        </div>
      </template>

      <el-form :model="form" label-position="top" class="main-form">
        <!-- 文件选择区域 -->
        <el-form-item label="选择视频文件" class="form-row">
          <div class="flex-row">
            <el-button type="primary" @click="onAddFolder">
              <el-icon>
                <Upload />
              </el-icon>
              选择单个视频文件
            </el-button>
            <el-button type="primary" @click="onAddFolderPath">
              <el-icon>
                <Upload />
              </el-icon>
              选择视频文件夹
            </el-button>
            <div v-if="form.input_path" class="file-info">
              <el-tag type="success" effect="dark">
                <el-icon>
                  <Upload />
                </el-icon> 已选择{{ form.is_folder ? '文件夹' : '视频' }}
              </el-tag>
            </div>
          </div>
        </el-form-item>

        <!-- 音频文件选择 -->
        <el-form-item label="选择配音文件" class="form-row">
          <div class="flex-row">
            <el-button type="primary" @click="onSelectAudio">
              <el-icon>
                <Headset />
              </el-icon>
              选择音频文件（不选择则不添加音频）
            </el-button>
            <div v-if="form.audio_path" class="file-info">
              <el-tag type="success" effect="dark" @click="onClearAudio">
                <el-icon>
                  <Headset />
                </el-icon> 已选择音频（点击后清除）
              </el-tag>
            </div>
          </div>
          <div class="tip-text">支持 MP3、WAV 等主流音频格式</div>
        </el-form-item>

        <!-- 文件夹模式特有选项 -->
        <template v-if="form.is_folder">
          <el-divider>高级设置</el-divider>

          <el-form-item label="剪辑顺序" class="form-row">
            <div class="flex-row">
              <el-radio-group v-model="form.order" class="order-group">
                <el-radio label="sequence">
                  <el-icon>
                    <Sort />
                  </el-icon> 按顺序
                </el-radio>
                <el-radio label="random">
                  <el-icon>
                    <Switch />
                  </el-icon> 随机
                </el-radio>
              </el-radio-group>
            </div>
          </el-form-item>

          <el-form-item label="视频生成时间" class="form-row">
            <div class="flex-row">
              <el-radio-group v-model="form.duration">
                <el-radio :label="40">
                  <el-icon>
                    <Timer />
                  </el-icon> 40秒
                </el-radio>
                <el-radio :label="50">
                  <el-icon>
                    <Timer />
                  </el-icon> 50秒
                </el-radio>
                <el-radio :label="60">
                  <el-icon>
                    <Timer />
                  </el-icon> 60秒
                </el-radio>
              </el-radio-group>
            </div>
          </el-form-item>
        </template>

        <!-- 操作按钮 -->
        <div class="submit-container">
          <el-button type="primary" class="submit-btn"
            :disabled="!form.input_path || (form.progress > 0 && form.progress < 100)" 
            @click="handleSubmit">
            <el-icon>
              <VideoPlay />
            </el-icon>
            {{ form.progress > 0 && form.progress < 100 
              ? `正在剪辑，剪辑进度：${form.progress}%${elapsedTime ? `，已用时：${elapsedTime}` : ''}` 
              : '开始剪辑' 
            }}
          </el-button>
          <span v-if="lastProcessTime" class="last-process-time">
            上次处理用时：{{ lastProcessTime }}
          </span>
        </div>
      </el-form>
    </el-card>
  </div>
</template>

<style scoped>
.container {
  min-height: 100vh;
  width: 100vw;
  display: flex;
  justify-content: center;
  align-items: center;
  background-color: #f0f2f5;
  padding: 20px;
  box-sizing: border-box;
  overflow-x: hidden;
}

.form-card {
  width: 100%;
  max-width: 800px;
  border-radius: 8px;
  box-shadow: 0 4px 24px rgba(0, 0, 0, 0.1);
}

.card-header {
  text-align: center;
  padding: 10px 0;
}

.card-header h2 {
  margin: 0;
  color: #303133;
  font-size: 24px;
}

.subtitle {
  margin: 8px 0 0;
  color: #909399;
  font-size: 14px;
}

.main-form {
  padding: 20px;
}

.form-row {
  margin-bottom: 24px;
}

.flex-row {
  display: flex;
  align-items: center;
  gap: 16px;
}

.file-info {
  flex: 1;
}

.tip-text {
  margin-top: 8px;
  color: #909399;
  font-size: 13px;
  padding-left: 4px;
}

.order-group {
  display: flex;
  gap: 32px;
}

.submit-container {
  display: flex;
  justify-content: center;
  align-items: center;
  margin-top: 32px;
  gap: 16px;
}

.submit-btn {
  width: auto;
  padding: 12px 36px;
  font-size: 16px;
}

.last-process-time {
  color: #909399;
  font-size: 14px;
}

:deep(.el-form-item__label) {
  font-weight: 500;
  font-size: 15px;
  margin-bottom: 8px;
}

:deep(.el-radio-group) {
  display: flex;
  gap: 32px;
}

:deep(.el-radio) {
  display: flex;
  align-items: center;
  margin-right: 0;
}

:deep(.el-divider) {
  margin: 32px 0;
}

:deep(.el-tag) {
  display: flex;
  align-items: center;
  gap: 5px;
  padding: 6px 12px;
  font-size: 14px;
}

:deep(.el-button .el-icon) {
  margin-right: 4px;
}
</style>
