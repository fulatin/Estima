<template>
  <div v-if="isOpen" class="fixed inset-0 bg-black/50 flex items-center justify-center z-50" @click.self="$emit('close')">
    <div class="bg-[#1e1e1e] border border-[#333] w-full max-w-md">
      <div class="flex items-center justify-between px-4 py-3 border-b border-[#333]">
        <h2 class="text-sm font-semibold text-[#a0a0a0] uppercase tracking-wider">Settings</h2>
        <button @click="$emit('close')" class="text-[#666] hover:text-white">
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>
      
      <div class="p-4 space-y-4">
        <div>
          <label class="block text-xs text-[#a0a0a0] mb-2">AI Provider</label>
          <div class="grid grid-cols-3 gap-2">
            <button 
              v-for="p in providers" 
              :key="p.value"
              @click="config.ai.provider = p.value"
              :class="[
                'py-2 text-xs border transition-colors',
                config.ai.provider === p.value 
                  ? 'border-amber-500 text-amber-400 bg-amber-500/10' 
                  : 'border-[#444] text-[#a0a0a0] hover:border-[#555]'
              ]"
            >
              {{ p.label }}
            </button>
          </div>
        </div>
        
        <div>
          <label class="block text-xs text-[#a0a0a0] mb-2">API Key</label>
          <input 
            v-model="config.ai.api_key"
            type="password"
            placeholder="Enter your API key..."
            class="w-full px-3 py-2 bg-[#121212] border border-[#333] text-white placeholder-[#666] focus:border-amber-500 focus:outline-none text-sm"
          >
        </div>
        
        <div>
          <label class="block text-xs text-[#a0a0a0] mb-2">Model (optional)</label>
          <input 
            v-model="config.ai.model"
            type="text"
            :placeholder="defaultModelPlaceholder"
            class="w-full px-3 py-2 bg-[#121212] border border-[#333] text-white placeholder-[#666] focus:border-amber-500 focus:outline-none text-sm"
          >
        </div>

        <div v-if="testResult" :class="['p-2 text-xs', testResult.success ? 'text-green-400 bg-green-400/10' : 'text-red-400 bg-red-400/10']">
          {{ testResult.message }}
        </div>
        
        <div class="flex gap-2 pt-2">
          <button 
            @click="testConnection"
            :disabled="testing || !config.ai.api_key || !config.ai.provider"
            class="flex-1 py-2 text-xs border border-[#444] text-[#a0a0a0] hover:border-[#555] disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            {{ testing ? 'Testing...' : 'Test Connection' }}
          </button>
          <button 
            @click="save"
            :disabled="saving"
            class="flex-1 py-2 text-xs bg-amber-500 text-black font-medium hover:bg-amber-400 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            {{ saving ? 'Saving...' : 'Save' }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'

defineProps<{ isOpen: boolean }>()
const emit = defineEmits<{ close: [] }>()

interface AIConfig {
  provider: string
  api_key: string
  model: string | null
  base_url: string | null
}

interface AppConfig {
  ai: AIConfig
}

const providers = [
  { value: 'deepseek', label: 'DeepSeek' },
  { value: 'siliconflow', label: 'SiliconFlow' },
  { value: 'openai', label: 'OpenAI' },
]

const config = ref<AppConfig>({
  ai: {
    provider: '',
    api_key: '',
    model: null,
    base_url: null,
  }
})

const saving = ref(false)
const testing = ref(false)
const testResult = ref<{ success: boolean; message: string } | null>(null)

const defaultModelPlaceholder = computed(() => {
  switch (config.value.ai.provider) {
    case 'deepseek': return 'deepseek-chat (default)'
    case 'siliconflow': return 'provider default'
    case 'openai': return 'gpt-4o (default)'
    default: return 'provider default'
  }
})

async function loadConfig() {
  try {
    const c = await invoke<AppConfig>('get_config')
    config.value = c
  } catch (e) {
    console.error('Failed to load config:', e)
  }
}

async function save() {
  saving.value = true
  testResult.value = null
  try {
    await invoke('save_config', { config: config.value })
    emit('close')
  } catch (e) {
    testResult.value = { success: false, message: `Save failed: ${e}` }
  } finally {
    saving.value = false
  }
}

async function testConnection() {
  testing.value = true
  testResult.value = null
  try {
    const result = await invoke<string>('test_ai_connection', { config: config.value.ai })
    testResult.value = { success: true, message: result }
  } catch (e) {
    testResult.value = { success: false, message: `${e}` }
  } finally {
    testing.value = false
  }
}

watch(() => config.value.ai.provider, () => {
  testResult.value = null
})

onMounted(loadConfig)
</script>
