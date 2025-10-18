import { useState, useEffect } from 'react';
import {
  Box,
  Container,
  Typography,
  Paper,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  TablePagination,
  Chip,
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  Button,
  CircularProgress,
  Avatar,
} from '@mui/material';

interface TokenDetail {
  id: number;
  bot_pubkey: string;
  bot_kind0_content: string | null;
  category_name: string;
  category_display_name: string;
  prompt_tokens: number;
  completion_tokens: number;
  total_tokens: number;
  prompt_text: string;
  completion_text: string;
  created_at: number;
}

export default function TokenDetailsPage() {
  const [details, setDetails] = useState<TokenDetail[]>([]);
  const [loading, setLoading] = useState(true);
  const [page, setPage] = useState(0);
  const [rowsPerPage, setRowsPerPage] = useState(25);
  const [total, setTotal] = useState(0);
  const [selectedDetail, setSelectedDetail] = useState<TokenDetail | null>(null);
  const [dialogOpen, setDialogOpen] = useState(false);

  const fetchDetails = async () => {
    setLoading(true);
    try {
      const response = await fetch(
        `/api/analytics/token-details?limit=${rowsPerPage}&offset=${page * rowsPerPage}`
      );
      const data = await response.json();
      setDetails(data.data);
      setTotal(data.total);
    } catch (error) {
      console.error('トークン詳細の取得に失敗:', error);
    }
    setLoading(false);
  };

  useEffect(() => {
    fetchDetails();
  }, [page, rowsPerPage]);

  const handleChangePage = (_event: unknown, newPage: number) => {
    setPage(newPage);
  };

  const handleChangeRowsPerPage = (event: React.ChangeEvent<HTMLInputElement>) => {
    setRowsPerPage(parseInt(event.target.value, 10));
    setPage(0);
  };

  const handleRowClick = (detail: TokenDetail) => {
    setSelectedDetail(detail);
    setDialogOpen(true);
  };

  const handleCloseDialog = () => {
    setDialogOpen(false);
  };

  const formatDate = (timestamp: number) => {
    return new Date(timestamp * 1000).toLocaleString('ja-JP');
  };

  const getCategoryColor = (categoryName: string) => {
    const colors: Record<string, 'primary' | 'secondary' | 'success' | 'warning' | 'info' | 'error'> = {
      reply: 'primary',
      air_reply: 'secondary',
      summary: 'success',
      search_initial_reply: 'info',
      search_keyword_extraction: 'warning',
      search_final_reply: 'error',
    };
    return colors[categoryName] || 'default';
  };

  const getBotInfo = (kind0Content: string | null) => {
    if (!kind0Content) {
      return { name: null, picture: null };
    }
    try {
      const parsed = JSON.parse(kind0Content);
      return {
        name: parsed.display_name || parsed.name || null,
        picture: parsed.picture || null,
      };
    } catch {
      return { name: null, picture: null };
    }
  };

  return (
    <Container maxWidth="xl" sx={{ mt: 4, mb: 4 }}>
      <Typography variant="h4" gutterBottom>
        トークン使用量詳細
      </Typography>

      <Paper sx={{ width: '100%', overflow: 'hidden', mt: 3 }}>
        {loading ? (
          <Box display="flex" justifyContent="center" p={3}>
            <CircularProgress />
          </Box>
        ) : (
          <>
            <TableContainer sx={{ maxHeight: 600 }}>
              <Table stickyHeader>
                <TableHead>
                  <TableRow>
                    <TableCell>日時</TableCell>
                    <TableCell>Bot</TableCell>
                    <TableCell>カテゴリ</TableCell>
                    <TableCell align="right">入力トークン</TableCell>
                    <TableCell align="right">出力トークン</TableCell>
                    <TableCell align="right">合計トークン</TableCell>
                  </TableRow>
                </TableHead>
                <TableBody>
                  {details.map((detail) => {
                    const botInfo = getBotInfo(detail.bot_kind0_content);
                    return (
                      <TableRow
                        key={detail.id}
                        hover
                        onClick={() => handleRowClick(detail)}
                        sx={{ cursor: 'pointer' }}
                      >
                        <TableCell>{formatDate(detail.created_at)}</TableCell>
                        <TableCell>
                          <Box display="flex" alignItems="center" gap={1}>
                            <Avatar
                              src={botInfo.picture || undefined}
                              sx={{ width: 32, height: 32 }}
                            >
                              {botInfo.name ? botInfo.name[0] : detail.bot_pubkey[0]}
                            </Avatar>
                            <Box>
                              {botInfo.name ? (
                                <>
                                  <Typography variant="body2">{botInfo.name}</Typography>
                                  <Typography variant="caption" color="text.secondary" fontFamily="monospace">
                                    {detail.bot_pubkey.substring(0, 8)}...
                                  </Typography>
                                </>
                              ) : (
                                <Typography variant="body2" fontFamily="monospace">
                                  {detail.bot_pubkey.substring(0, 8)}...
                                </Typography>
                              )}
                            </Box>
                          </Box>
                        </TableCell>
                        <TableCell>
                          <Chip
                            label={detail.category_display_name}
                            color={getCategoryColor(detail.category_name)}
                            size="small"
                          />
                        </TableCell>
                        <TableCell align="right">{detail.prompt_tokens.toLocaleString()}</TableCell>
                        <TableCell align="right">{detail.completion_tokens.toLocaleString()}</TableCell>
                        <TableCell align="right">
                          <Typography variant="body2" fontWeight="bold">
                            {detail.total_tokens.toLocaleString()}
                          </Typography>
                        </TableCell>
                      </TableRow>
                    );
                  })}
                </TableBody>
              </Table>
            </TableContainer>
            <TablePagination
              rowsPerPageOptions={[10, 25, 50, 100]}
              component="div"
              count={total}
              rowsPerPage={rowsPerPage}
              page={page}
              onPageChange={handleChangePage}
              onRowsPerPageChange={handleChangeRowsPerPage}
              labelRowsPerPage="表示件数:"
              labelDisplayedRows={({ from, to, count }) => `${from}-${to} / ${count}件`}
            />
          </>
        )}
      </Paper>

      {/* 詳細ダイアログ */}
      <Dialog
        open={dialogOpen}
        onClose={handleCloseDialog}
        maxWidth="md"
        fullWidth
      >
        {selectedDetail && (
          <>
            <DialogTitle>
              <Box display="flex" justifyContent="space-between" alignItems="center">
                <Typography variant="h6">トークン使用量詳細</Typography>
                <Chip
                  label={selectedDetail.category_display_name}
                  color={getCategoryColor(selectedDetail.category_name)}
                />
              </Box>
            </DialogTitle>
            <DialogContent dividers>
              <Box mb={2}>
                <Typography variant="subtitle2" color="text.secondary" gutterBottom>
                  日時
                </Typography>
                <Typography variant="body1">{formatDate(selectedDetail.created_at)}</Typography>
              </Box>
              
              <Box mb={2}>
                <Typography variant="subtitle2" color="text.secondary" gutterBottom>
                  Bot公開鍵
                </Typography>
                <Typography variant="body2" fontFamily="monospace">
                  {selectedDetail.bot_pubkey}
                </Typography>
              </Box>

              <Box mb={2}>
                <Typography variant="subtitle2" color="text.secondary" gutterBottom>
                  トークン数
                </Typography>
                <Typography variant="body1">
                  入力: {selectedDetail.prompt_tokens} / 出力: {selectedDetail.completion_tokens} / 合計: {selectedDetail.total_tokens}
                </Typography>
              </Box>

              <Box mb={2}>
                <Typography variant="subtitle2" color="text.secondary" gutterBottom>
                  入力テキスト（プロンプト）
                </Typography>
                <Paper variant="outlined" sx={{ p: 2, bgcolor: 'grey.50' }}>
                  <Typography
                    variant="body2"
                    component="pre"
                    sx={{
                      whiteSpace: 'pre-wrap',
                      wordBreak: 'break-word',
                      fontFamily: 'monospace',
                      fontSize: '0.875rem',
                    }}
                  >
                    {selectedDetail.prompt_text}
                  </Typography>
                </Paper>
              </Box>

              <Box>
                <Typography variant="subtitle2" color="text.secondary" gutterBottom>
                  出力テキスト（完了）
                </Typography>
                <Paper variant="outlined" sx={{ p: 2, bgcolor: 'grey.50' }}>
                  <Typography
                    variant="body2"
                    component="pre"
                    sx={{
                      whiteSpace: 'pre-wrap',
                      wordBreak: 'break-word',
                      fontFamily: 'monospace',
                      fontSize: '0.875rem',
                    }}
                  >
                    {selectedDetail.completion_text}
                  </Typography>
                </Paper>
              </Box>
            </DialogContent>
            <DialogActions>
              <Button onClick={handleCloseDialog}>閉じる</Button>
            </DialogActions>
          </>
        )}
      </Dialog>
    </Container>
  );
}

