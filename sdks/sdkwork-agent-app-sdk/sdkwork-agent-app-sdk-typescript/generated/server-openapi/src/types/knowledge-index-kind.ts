/** Retrieval index mode. RAG is not vector-only; exact, keyword, full_text, structured, graph, wiki, rule, hybrid, llm_rerank, and external indexes are first-class modes. */
export type KnowledgeIndexKind = 'exact' | 'keyword' | 'full_text' | 'structured' | 'graph' | 'wiki' | 'rule' | 'vector' | 'hybrid' | 'llm_rerank' | 'external';
