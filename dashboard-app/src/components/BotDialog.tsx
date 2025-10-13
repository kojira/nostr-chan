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
} from '@mui/material';
import { VpnKey, Psychology, Description, Save, Close } from '@mui/icons-material';
import type { BotData, BotRequest } from '../types';

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
    content: '',
  });

  useEffect(() => {
    if (bot) {
      setFormData({
        secretkey: bot.secretkey || '',
        prompt: bot.prompt || '',
        content: bot.content || '',
      });
    } else {
      setFormData({ secretkey: '', prompt: '', content: '' });
    }
  }, [bot, open]);

  const handleSubmit = (e) => {
    e.preventDefault();
    onSave(formData, bot?.pubkey);
  };

  return (
    <Dialog open={open} onClose={onClose} maxWidth="md" fullWidth>
      <DialogTitle>
        {bot ? '✏️ Bot編集' : '➕ Bot追加'}
      </DialogTitle>
      <form onSubmit={handleSubmit}>
        <DialogContent>
          <Box sx={{ display: 'flex', flexDirection: 'column', gap: 3 }}>
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
            
            <TextField
              label="追加情報（content）"
              value={formData.content}
              onChange={(e) => setFormData({ ...formData, content: e.target.value })}
              fullWidth
              multiline
              rows={4}
              placeholder="追加の設定やメモ（任意）"
              InputProps={{
                startAdornment: (
                  <InputAdornment position="start" sx={{ alignSelf: 'flex-start', mt: 2 }}>
                    <Description />
                  </InputAdornment>
                ),
              }}
            />
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

