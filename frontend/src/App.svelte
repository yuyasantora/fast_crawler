<script>
  const API_BASE = import.meta.env.VITE_API_BASE;
  const PER_PAGE = 10;

  let keyword = '';
  let results = [];
  let page = 0;
  let loading = false;
  let analyzing = null;
  let error = '';

  $: totalPages = Math.ceil(results.length / PER_PAGE);
  $: pagedResults = results.slice(page * PER_PAGE, (page + 1) * PER_PAGE);

  async function search() {
    if (!keyword) {
      error = 'キーワードを入力してください';
      return;
    }

    loading = true;
    error = '';
    results = [];
    page = 0;

    try {
      const res = await fetch(`${API_BASE}/search`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ keyword, limit: 100 })
      });
      const data = await res.json();

      if (data.success) {
        results = data.results;
      } else {
        error = data.error || '検索に失敗しました';
      }
    } catch (e) {
      error = `接続エラー: ${e.message}`;
    } finally {
      loading = false;
    }
  }

  async function analyze(caseId) {
    analyzing = caseId;
    error = '';

    try {
      const res = await fetch(`${API_BASE}/analyze`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ case_id: caseId })
      });
      const data = await res.json();

      if (data.success) {
        downloadPdf(caseId);
      } else {
        error = data.error || '分析に失敗しました';
      }
    } catch (e) {
      error = `分析エラー: ${e.message}`;
    } finally {
      analyzing = null;
    }
  }

  function downloadPdf(caseId) {
    window.open(`${API_BASE}/pdf/${caseId}`, '_blank');
  }
</script>

<main>
  <h1>My Legal Engine</h1>
  <p class="subtitle">特許判決検索・分析システム</p>

  <div class="search-box">
    <input
      type="text"
      bind:value={keyword}
      placeholder="キーワード（例: 特許権侵害）"
      on:keydown={(e) => e.key === 'Enter' && search()}
    />
    <button on:click={search} disabled={loading}>
      {loading ? '検索中...' : '検索'}
    </button>
  </div>

  {#if error}
    <div class="error">{error}</div>
  {/if}

  {#if analyzing}
    <div class="loading-overlay">
      <div class="spinner"></div>
      <p>AI分析中...</p>
    </div>
  {/if}

  {#if results.length > 0}
    <div class="results">
      <h2>検索結果 ({results.length}件)</h2>
      <table>
        <thead>
          <tr>
            <th>タイトル</th>
            <th>操作</th>
          </tr>
        </thead>
        <tbody>
          {#each pagedResults as r}
            <tr>
              <td>{r.title}</td>
              <td>
                <button
                  class="analyze-btn"
                  on:click={() => analyze(r.case_id)}
                  disabled={analyzing === r.case_id}
                >
                  {analyzing === r.case_id ? '分析中...' : '分析・PDF'}
                </button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>

      {#if totalPages > 1}
        <div class="pagination">
          <button on:click={() => page--} disabled={page === 0}>前へ</button>
          <span>{page + 1} / {totalPages}</span>
          <button on:click={() => page++} disabled={page >= totalPages - 1}>次へ</button>
        </div>
      {/if}
    </div>
  {/if}
</main>

<style>
  main {
    max-width: 1000px;
    margin: 0 auto;
    padding: 1rem 2rem;
  }

  h1 {
    color: #333;
    margin: 0 0 0.25rem 0;
  }

  .subtitle {
    color: #666;
    margin: 0 0 1.5rem 0;
  }

  .search-box {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 1rem;
  }

  .search-box input {
    flex: 1;
    padding: 0.75rem;
    border: 1px solid #ccc;
    border-radius: 4px;
    font-size: 1rem;
  }

  .search-box button {
    padding: 0.75rem 1.5rem;
    background: #4a90d9;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 1rem;
  }

  .search-box button:hover:not(:disabled) {
    background: #357abd;
  }

  .search-box button:disabled {
    background: #999;
    cursor: not-allowed;
  }

  .error {
    background: #fee;
    color: #c00;
    padding: 1rem;
    border-radius: 4px;
    margin-bottom: 1rem;
  }

  .results h2 {
    margin-bottom: 1rem;
  }

  table {
    width: 100%;
    border-collapse: collapse;
  }

  th, td {
    padding: 0.75rem;
    text-align: left;
    border-bottom: 1px solid #eee;
  }

  th {
    background: #f5f5f5;
    font-weight: 600;
  }

  .title-cell {
    max-width: 400px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .analyze-btn {
    padding: 0.5rem 1rem;
    background: #28a745;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.875rem;
  }

  .analyze-btn:hover:not(:disabled) {
    background: #218838;
  }

  .analyze-btn:disabled {
    background: #999;
    cursor: not-allowed;
  }

  .pagination {
    display: flex;
    justify-content: center;
    align-items: center;
    gap: 1rem;
    margin-top: 1rem;
    padding: 1rem 0;
  }

  .pagination button {
    padding: 0.5rem 1rem;
    background: #4a90d9;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
  }

  .pagination button:hover:not(:disabled) {
    background: #357abd;
  }

  .pagination button:disabled {
    background: #ccc;
    cursor: not-allowed;
  }

  .loading-overlay {
    position: fixed;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    z-index: 1000;
  }

  .loading-overlay p {
    color: white;
    margin-top: 1rem;
    font-size: 1.2rem;
  }

  .spinner {
    width: 50px;
    height: 50px;
    border: 4px solid #fff;
    border-top: 4px solid #4a90d9;
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }
</style>
