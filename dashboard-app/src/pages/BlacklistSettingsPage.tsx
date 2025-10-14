import { useState, useEffect } from 'react';
import {
  Container, Box, Typography, IconButton, Paper, Button, TextField,
  List, ListItem, ListItemText, ListItemSecondaryAction, Chip, Avatar, ListItemAvatar,
  Snackbar, Alert
} from '@mui/material';
import { ArrowBack, Save, Block, Add, Delete, Person } from '@mui/icons-material';
import { useNavigate } from 'react-router-dom';
import { nip19 } from 'nostr-tools';

interface BlacklistEntry {
  pubkey: string;
  name: string;
  picture?: string;
}

const hexToNpub = (hex: string): string => {
  try {
    return nip19.npubEncode(hex);
  } catch (error) {
    console.error('hex→npub変換エラー:', error);
    return hex;
  }
};

export const BlacklistSettingsPage = () => {
  const navigate = useNavigate();
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [blacklist, setBlacklist] = useState<BlacklistEntry[]>([]);
  const [newPubkey, setNewPubkey] = useState('');
  const [snackbar, setSnackbar] = useState<{ open: boolean; message: string; severity: 'success' | 'error' }>({
    open: false,
    message: '',
    severity: 'success',
  });

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

  const convertNpubToHex = (input: string): string | null => {
    try {
      // npub形式かどうかをチェック
      if (input.startsWith('npub1')) {
        const decoded = nip19.decode(input);
        if (decoded.type === 'npub') {
          return decoded.data;
        }
      }
      // hex形式の場合はそのまま返す
      if (/^[0-9a-fA-F]{64}$/.test(input)) {
        return input.toLowerCase();
      }
      return null;
    } catch (error) {
      console.error('npub変換エラー:', error);
      return null;
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
        setSnackbar({
          open: true,
          message: '設定を保存しました',
          severity: 'success',
        });
      } else {
        setSnackbar({
          open: true,
          message: '設定の保存に失敗しました',
          severity: 'error',
        });
      }
    } catch (error) {
      console.error('保存エラー:', error);
      setSnackbar({
        open: true,
        message: '設定の保存に失敗しました',
        severity: 'error',
      });
    } finally {
      setSaving(false);
    }
  };

  const addPubkey = (input: string) => {
    const hexPubkey = convertNpubToHex(input);
    
    if (!hexPubkey) {
      setSnackbar({
        open: true,
        message: '公開鍵は64文字の16進数、またはnpub1形式で入力してください',
        severity: 'error',
      });
      return;
    }

    if (blacklist.some(entry => entry.pubkey === hexPubkey)) {
      setSnackbar({
        open: true,
        message: 'この公開鍵は既にブラックリストに登録されています',
        severity: 'error',
      });
      return;
    }

    // 新しいエントリを追加（名前とアイコンは後で更新される）
    const newEntry: BlacklistEntry = {
      pubkey: hexPubkey,
      name: `${hexPubkey.substring(0, 8)}...`,
      picture: undefined,
    };
    
    setBlacklist([...blacklist, newEntry]);
    setNewPubkey('');
    setSnackbar({
      open: true,
      message: 'ブラックリストに追加しました',
      severity: 'success',
    });
  };

  const removePubkey = (pubkey: string) => {
    setBlacklist(blacklist.filter(entry => entry.pubkey !== pubkey));
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
          {blacklist.map((entry, index) => (
            <ListItem key={index} sx={{ bgcolor: 'grey.50', mb: 1, borderRadius: 1 }}>
              <ListItemAvatar>
                <Avatar src={entry.picture} sx={{ bgcolor: 'primary.main' }}>
                  {entry.picture ? null : <Person />}
                </Avatar>
              </ListItemAvatar>
              <ListItemText 
                primary={
                  <Box>
                    <Typography variant="subtitle2" fontWeight="bold">
                      {entry.name}
                    </Typography>
                    <Typography 
                      variant="caption" 
                      sx={{ 
                        fontFamily: 'monospace',
                        color: 'text.secondary',
                        wordBreak: 'break-all',
                        display: 'block',
                        mb: 0.5,
                      }}
                    >
                      {hexToNpub(entry.pubkey)}
                    </Typography>
                    <Typography 
                      variant="caption" 
                      sx={{ 
                        fontFamily: 'monospace',
                        color: 'text.disabled',
                        wordBreak: 'break-all',
                        display: 'block',
                        fontSize: '0.7rem',
                      }}
                    >
                      {entry.pubkey}
                    </Typography>
                  </Box>
                }
              />
              <ListItemSecondaryAction>
                <IconButton edge="end" onClick={() => removePubkey(entry.pubkey)} size="small" color="error">
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
            placeholder="公開鍵（hex形式 または npub1...）"
            value={newPubkey}
            onChange={(e) => setNewPubkey(e.target.value.trim())}
            onKeyPress={(e) => e.key === 'Enter' && newPubkey && addPubkey(newPubkey)}
            inputProps={{
              style: { fontFamily: 'monospace', fontSize: '0.875rem' }
            }}
          />
          <Button
            variant="outlined"
            startIcon={<Add />}
            onClick={() => addPubkey(newPubkey)}
            disabled={!newPubkey}
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
            <li>公開鍵は <strong>64文字の16進数</strong>（例: 1234abcd...）、または <strong>npub1形式</strong>（例: npub1...）で入力できます</li>
            <li>npub形式で入力した場合、自動的に16進数に変換されます</li>
            <li>ユーザー名とアイコンは、過去の投稿情報から自動取得されます</li>
            <li>ブラックリストに登録されたユーザーからの投稿は、Botが一切反応しなくなります</li>
            <li>設定は保存後すぐに反映されます（再起動不要）</li>
          </ul>
        </Typography>
      </Paper>

      {/* Snackbar通知 */}
      <Snackbar
        open={snackbar.open}
        autoHideDuration={4000}
        onClose={() => setSnackbar({ ...snackbar, open: false })}
        anchorOrigin={{ vertical: 'bottom', horizontal: 'center' }}
      >
        <Alert
          onClose={() => setSnackbar({ ...snackbar, open: false })}
          severity={snackbar.severity}
          variant="filled"
          sx={{ width: '100%' }}
        >
          {snackbar.message}
        </Alert>
      </Snackbar>
    </Container>
  );
};

