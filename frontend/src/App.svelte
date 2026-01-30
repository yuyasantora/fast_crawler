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
    loading = true;
    error = '';
    results = [];
    page = 0;

    try {
      const res = await fetch(`${API_BASE}/search`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ keyword: keyword || null, limit: 100 })
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
  <div class="header">
    <h1>Patent Researcher</h1>
    <p class="subtitle">特許判決検索・分析システム</p>
  </div>

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
      <p>AI分析中 + PDF生成中...</p>
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

  .header {
    text-align: center;
    margin-bottom: 2rem;
  }

  h1 {
    color: #fff;
    margin: 0 0 0.5rem 0;
    font-weight: 700;
    font-size: 2.5rem;
    letter-spacing: 0.02em;
  }

  .subtitle {
    color: #888;
    margin: 0;
    font-size: 0.9rem;
  }

  .search-box {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 1.5rem;
  }

  .search-box input {
    flex: 1;
    padding: 0.75rem 1rem;
    background: #2a2a2a;
    border: 1px solid #444;
    border-radius: 6px;
    font-size: 1rem;
    color: #e0e0e0;
    transition: border-color 0.2s;
  }

  .search-box input:focus {
    outline: none;
    border-color: #6a9fd9;
  }

  .search-box input::placeholder {
    color: #666;
  }

  .search-box button {
    padding: 0.75rem 1.5rem;
    background: linear-gradient(135deg, #4a90d9, #357abd);
    color: white;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 1rem;
    transition: transform 0.1s, box-shadow 0.2s;
  }

  .search-box button:hover:not(:disabled) {
    transform: translateY(-1px);
    box-shadow: 0 4px 12px rgba(74, 144, 217, 0.3);
  }

  .search-box button:disabled {
    background: #444;
    cursor: not-allowed;
  }

  .error {
    background: #3a2020;
    color: #ff6b6b;
    padding: 1rem;
    border-radius: 6px;
    margin-bottom: 1rem;
    border: 1px solid #4a2020;
  }

  .results h2 {
    margin-bottom: 1rem;
    color: #ccc;
    font-weight: 400;
  }

  table {
    width: 100%;
    border-collapse: collapse;
  }

  th, td {
    padding: 0.875rem;
    text-align: left;
    border-bottom: 1px solid #333;
  }

  th {
    background: #252525;
    font-weight: 500;
    color: #aaa;
    text-transform: uppercase;
    font-size: 0.8rem;
    letter-spacing: 0.05em;
  }

  tbody tr:hover {
    background: #252525;
  }

  .analyze-btn {
    padding: 0.5rem 1rem;
    background: linear-gradient(135deg, #2ecc71, #27ae60);
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.8rem;
    transition: transform 0.1s, box-shadow 0.2s;
  }

  .analyze-btn:hover:not(:disabled) {
    transform: translateY(-1px);
    box-shadow: 0 4px 12px rgba(46, 204, 113, 0.3);
  }

  .analyze-btn:disabled {
    background: #444;
    cursor: not-allowed;
  }

  .pagination {
    display: flex;
    justify-content: center;
    align-items: center;
    gap: 1rem;
    margin-top: 1.5rem;
    padding: 1rem 0;
  }

  .pagination span {
    color: #888;
  }

  .pagination button {
    padding: 0.5rem 1rem;
    background: #333;
    color: #ccc;
    border: 1px solid #444;
    border-radius: 4px;
    cursor: pointer;
    transition: background 0.2s;
  }

  .pagination button:hover:not(:disabled) {
    background: #444;
  }

  .pagination button:disabled {
    background: #222;
    color: #555;
    cursor: not-allowed;
  }

  .loading-overlay {
    position: fixed;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    background: rgba(0, 0, 0, 0.85);
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    z-index: 1000;
  }

  .loading-overlay p {
    color: #ccc;
    margin-top: 1.5rem;
    font-size: 1rem;
    letter-spacing: 0.05em;
  }

  .spinner {
    width: 48px;
    height: 48px;
    border: 3px solid #333;
    border-top: 3px solid #4a90d9;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
  }
</style>
