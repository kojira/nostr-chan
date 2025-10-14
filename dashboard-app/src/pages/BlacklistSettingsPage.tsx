import { useState, useEffect } from 'react';
import {
  Container, Box, Typography, IconButton, Paper, Button, TextField,
  List, ListItem, ListItemText, ListItemSecondaryAction, Chip
} from '@mui/material';
import { ArrowBack, Save, Block, Add, Delete } from '@mui/icons-material';
import { useNavigate } from 'react-router-dom';

export const BlacklistSettingsPage = () => {
  const navigate = useNavigate();
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [blacklist, setBlacklist] = useState<string[]>([]);
  const [newPubkey, setNewPubkey] = useState('');

  useEffect(() => {
    loadSettings();
  }, []);

  const loadSettings = async () => {
    try {
      const response = await fetch('/api/settings/blacklist');
      if (response.ok) {
        const data = await response.json();
        setBlacklist(data.blacklist || []);
      }
    } catch (error) {
      console.error('設定読み込みエラー:', error);
    } finally {
      setLoading(false);
    }
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      const response = await fetch('/api/settings/blacklist', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          blacklist: blacklist,
        }),
      });

      if (response.ok) {
        alert('✅ 設定を保存しました');
      } else {
        alert('❌ 設定の保存に失敗しました');
      }
    } catch (error) {
      console.error('保存エラー:', error);
      alert('❌ 設定の保存に失敗しました');
    } finally {
      setSaving(false);
    }
  };

  const addPubkey = (pubkey: string) => {
    if (pubkey.length !== 64) {
      alert('公開鍵は64文字の16進数である必要があります');
      return;
    }

    if (!/^[0-9a-fA-F]{64}$/.test(pubkey)) {
      alert('公開鍵は16進数（0-9, a-f）のみで構成される必要があります');
      return;
    }

    if (!blacklist.includes(pubkey)) {
      setBlacklist([...blacklist, pubkey]);
      setNewPubkey('');
    } else {
      alert('この公開鍵は既にブラックリストに登録されています');
    }
  };

  const removePubkey = (pubkey: string) => {
    setBlacklist(blacklist.filter(p => p !== pubkey));
  };

  if (loading) {
    return null;
  }

  return (
    <Container maxWidth="md" sx={{ py: 4 }}>
      {/* ヘッダー */}
      <Box sx={{ display: 'flex', alignItems: 'center', gap: 2, mb: 4 }}>
        <IconButton onClick={() => navigate('/')} size="large">
          <ArrowBack />
        </IconButton>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, flex: 1 }}>
          <Block sx={{ fontSize: 32 }} />
          <Typography variant="h4" fontWeight="bold">
            ブラックリスト設定
          </Typography>
        </Box>
        <Button
          variant="contained"
          startIcon={<Save />}
          onClick={handleSave}
          disabled={saving}
          size="large"
        >
          {saving ? '保存中...' : '保存'}
        </Button>
      </Box>

      {/* ブラックリスト */}
      <Paper sx={{ p: 3, mb: 3 }}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 2 }}>
          <Typography variant="h6" fontWeight="bold">
            ブラックリスト
          </Typography>
          <Chip label={`${blacklist.length}件`} size="small" color="error" />
        </Box>
        
        <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
          ブラックリストに登録されたユーザーからの投稿は無視されます。
        </Typography>

        <List>
          {blacklist.map((pubkey, index) => (
            <ListItem key={index} sx={{ bgcolor: 'grey.50', mb: 1, borderRadius: 1 }}>
              <ListItemText 
                primary={pubkey} 
                primaryTypographyProps={{ 
                  fontFamily: 'monospace',
                  fontSize: '0.875rem',
                  sx: { wordBreak: 'break-all' }
                }} 
              />
              <ListItemSecondaryAction>
                <IconButton edge="end" onClick={() => removePubkey(pubkey)} size="small" color="error">
                  <Delete />
                </IconButton>
              </ListItemSecondaryAction>
            </ListItem>
          ))}
          {blacklist.length === 0 && (
            <Box sx={{ textAlign: 'center', py: 4, color: 'text.secondary' }}>
              ブラックリストは空です
            </Box>
          )}
        </List>

        <Box sx={{ display: 'flex', gap: 1, mt: 2 }}>
          <TextField
            fullWidth
            size="small"
            placeholder="64文字の16進数公開鍵"
            value={newPubkey}
            onChange={(e) => setNewPubkey(e.target.value.toLowerCase())}
            onKeyPress={(e) => e.key === 'Enter' && addPubkey(newPubkey)}
            inputProps={{
              style: { fontFamily: 'monospace' }
            }}
          />
          <Button
            variant="outlined"
            startIcon={<Add />}
            onClick={() => addPubkey(newPubkey)}
            disabled={!newPubkey || newPubkey.length !== 64}
          >
            追加
          </Button>
        </Box>
      </Paper>

      {/* 使い方 */}
      <Paper sx={{ p: 3, bgcolor: 'info.light' }}>
        <Typography variant="subtitle1" fontWeight="bold" gutterBottom>
          💡 使い方
        </Typography>
        <Typography variant="body2" component="div">
          <ul style={{ margin: 0, paddingLeft: 20 }}>
            <li>公開鍵は64文字の16進数（0-9, a-f）で入力してください</li>
            <li>npub形式の場合は、16進数に変換してから入力してください</li>
            <li>ブラックリストに登録されたユーザーからの投稿は、Botが一切反応しなくなります</li>
            <li>設定は保存後すぐに反映されます（再起動不要）</li>
          </ul>
        </Typography>
      </Paper>
    </Container>
  );
};

