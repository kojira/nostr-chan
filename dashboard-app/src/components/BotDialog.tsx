import { useState, useEffect } from 'react';
import {
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  TextField,
  Button,
  Box,
  InputAdornment,
  IconButton,
  Typography,
} from '@mui/material';
import { VpnKey, Psychology, Description, Save, Close, Add, Delete, CloudDownload, Casino } from '@mui/icons-material';
import type { BotData, BotRequest } from '../types';

interface JsonField {
  key: string;
  value: string;
}

interface BotDialogProps {
  open: boolean;
  bot: BotData | null;
  onClose: () => void;
  onSave: (data: BotRequest, pubkey?: string) => void;
}

export const BotDialog = ({ open, bot, onClose, onSave }: BotDialogProps) => {
  const [formData, setFormData] = useState({
    secretkey: '',
    prompt: '',
  });
  const [jsonFields, setJsonFields] = useState<JsonField[]>([{ key: '', value: '' }]);
  const [fetchingKind0, setFetchingKind0] = useState(false);

  useEffect(() => {
    if (bot) {
      setFormData({
        secretkey: bot.secretkey || '',
        prompt: bot.prompt || '',
      });
      
      // contentをJSONとしてパースしてフィールドに展開
      if (bot.content) {
        try {
          const parsed = JSON.parse(bot.content);
          const fields = Object.entries(parsed).map(([key, value]) => ({
            key,
            value: String(value),
          }));
          setJsonFields(fields.length > 0 ? fields : [{ key: '', value: '' }]);
        } catch {
          setJsonFields([{ key: '', value: '' }]);
        }
      } else {
        setJsonFields([{ key: '', value: '' }]);
      }
    } else {
      setFormData({ secretkey: '', prompt: '' });
      setJsonFields([{ key: '', value: '' }]);
    }
  }, [bot, open]);

  const handleAddField = () => {
    setJsonFields([...jsonFields, { key: '', value: '' }]);
  };

  const handleRemoveField = (index: number) => {
    setJsonFields(jsonFields.filter((_, i) => i !== index));
  };

  const handleFieldChange = (index: number, field: 'key' | 'value', value: string) => {
    const newFields = [...jsonFields];
    newFields[index][field] = value;
    setJsonFields(newFields);
  };

  const handleGenerateSecretKey = async () => {
    try {
      const response = await fetch('/api/bots/generate-key');
      if (!response.ok) {
        throw new Error('秘密鍵の生成に失敗しました');
      }

      const data = await response.json();
      setFormData({ ...formData, secretkey: data.secretkey });
    } catch (error) {
      console.error('秘密鍵生成エラー:', error);
      alert('❌ 秘密鍵を生成できませんでした');
    }
  };

  const handleFetchKind0 = async () => {
    if (!bot?.pubkey) {
      alert('❌ Botが選択されていません');
      return;
    }

    setFetchingKind0(true);
    try {
      const response = await fetch(`/api/bots/${bot.pubkey}/kind0`);
      if (!response.ok) {
        throw new Error('Kind 0の取得に失敗しました');
      }

      const data = await response.json();
      if (data.content) {
        try {
          const parsed = JSON.parse(data.content);
          const fields = Object.entries(parsed).map(([key, value]) => ({
            key,
            value: String(value),
          }));
          setJsonFields(fields.length > 0 ? fields : [{ key: '', value: '' }]);
        } catch (e) {
          alert('❌ 取得したKind 0のパースに失敗しました');
        }
      }
    } catch (error) {
      console.error('Kind 0取得エラー:', error);
      alert('❌ リレーからKind 0を取得できませんでした');
    } finally {
      setFetchingKind0(false);
    }
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    
    // JSONフィールドをオブジェクトに変換
    const contentObj: Record<string, string> = {};
    jsonFields.forEach(({ key, value }) => {
      if (key.trim()) {
        contentObj[key] = value;
      }
    });
    
    const content = Object.keys(contentObj).length > 0 ? JSON.stringify(contentObj) : '';
    
    onSave({ ...formData, content }, bot?.pubkey);
  };

  return (
    <Dialog open={open} onClose={onClose} maxWidth="md" fullWidth>
      <DialogTitle>
        {bot ? '✏️ Bot編集' : '➕ Bot追加'}
      </DialogTitle>
      <form onSubmit={handleSubmit}>
        <DialogContent>
          <Box sx={{ display: 'flex', flexDirection: 'column', gap: 3 }}>
            <Box sx={{ display: 'flex', gap: 1 }}>
              <TextField
                label="Secret Key"
                value={formData.secretkey}
                onChange={(e) => setFormData({ ...formData, secretkey: e.target.value })}
                required
                fullWidth
                placeholder="nsec1... または hex形式"
                InputProps={{
                  startAdornment: (
                    <InputAdornment position="start">
                      <VpnKey />
                    </InputAdornment>
                  ),
                }}
              />
              {!bot && (
                <Button
                  startIcon={<Casino />}
                  onClick={handleGenerateSecretKey}
                  variant="outlined"
                  sx={{ minWidth: '140px', whiteSpace: 'nowrap' }}
                >
                  ランダム生成
                </Button>
              )}
            </Box>
            
            <TextField
              label="プロンプト（Bot性格設定）"
              value={formData.prompt}
              onChange={(e) => setFormData({ ...formData, prompt: e.target.value })}
              required
              fullWidth
              multiline
              rows={6}
              placeholder="Botのキャラクター、話し方、応答スタイルなどを記述"
              InputProps={{
                startAdornment: (
                  <InputAdornment position="start" sx={{ alignSelf: 'flex-start', mt: 2 }}>
                    <Psychology />
                  </InputAdornment>
                ),
              }}
            />
            
            <Box>
              <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', mb: 2 }}>
                <Typography variant="subtitle1" fontWeight="bold">
                  追加情報（content - JSON）
                </Typography>
                <Box sx={{ display: 'flex', gap: 1 }}>
                  {bot && (
                    <Button 
                      startIcon={<CloudDownload />} 
                      onClick={handleFetchKind0} 
                      size="small"
                      variant="outlined"
                      disabled={fetchingKind0}
                    >
                      {fetchingKind0 ? '取得中...' : 'リレーから取得'}
                    </Button>
                  )}
                  <Button startIcon={<Add />} onClick={handleAddField} size="small">
                    フィールド追加
                  </Button>
                </Box>
              </Box>
              
              {jsonFields.map((field, index) => (
                <Box key={index} sx={{ display: 'flex', gap: 2, mb: 2 }}>
                  <TextField
                    label="キー"
                    value={field.key}
                    onChange={(e) => handleFieldChange(index, 'key', e.target.value)}
                    placeholder="name"
                    sx={{ flex: 1 }}
                    size="small"
                  />
                  <TextField
                    label="値"
                    value={field.value}
                    onChange={(e) => handleFieldChange(index, 'value', e.target.value)}
                    placeholder="Bot名"
                    sx={{ flex: 2 }}
                    size="small"
                    multiline
                  />
                  <IconButton 
                    onClick={() => handleRemoveField(index)} 
                    disabled={jsonFields.length === 1}
                    color="error"
                    size="small"
                  >
                    <Delete />
                  </IconButton>
                </Box>
              ))}
              
              <Typography variant="caption" color="text.secondary">
                Kind 0メタデータとして保存されます（例: name, about, picture など）
              </Typography>
            </Box>
          </Box>
        </DialogContent>
        
        <DialogActions sx={{ px: 3, pb: 3 }}>
          <Button onClick={onClose} startIcon={<Close />}>
            キャンセル
          </Button>
          <Button type="submit" variant="contained" startIcon={<Save />}>
            保存
          </Button>
        </DialogActions>
      </form>
    </Dialog>
  );
};

