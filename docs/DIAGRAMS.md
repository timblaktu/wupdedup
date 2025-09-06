# wupdedup-rs System Diagrams

## 1. High-Level System Architecture

```mermaid
graph TB
    subgraph "User Interface Layer"
        CLI[CLI Commands]
        API[REST API<br/>Future]
        WEB[Web UI<br/>Future]
    end
    
    subgraph "Application Core"
        SCAN[Scanner Engine]
        DEDUP[Deduplication Engine]
        COMP[Comparison Engine]
        STATS[Statistics Engine]
    end
    
    subgraph "Storage Abstraction Layer"
        STRAT[Storage Strategy Interface]
        LOCAL[Local Storage]
        SMUG[SmugMug API]
        S3[S3 Storage]
        GCS[Google Cloud Storage]
    end
    
    subgraph "Data Layer"
        DB[(redb Database)]
        CACHE[Memory Cache]
        CONFIG[Configuration]
    end
    
    subgraph "Supporting Services"
        LOG[Logging Service]
        PROF[Profiler]
        HASH[Hash Calculator]
        CONT[Content Analyzer]
    end
    
    CLI --> SCAN
    CLI --> DEDUP
    CLI --> COMP
    CLI --> STATS
    
    SCAN --> STRAT
    DEDUP --> DB
    COMP --> DB
    STATS --> DB
    
    STRAT --> LOCAL
    STRAT --> SMUG
    STRAT --> S3
    STRAT --> GCS
    
    SCAN --> HASH
    SCAN --> CONT
    SCAN --> DB
    
    LOCAL --> HASH
    SMUG --> CACHE
    S3 --> CACHE
```

## 2. Storage Strategy Pattern

```mermaid
classDiagram
    class StorageStrategy {
        <<interface>>
        +scan_tree(context) Result
        +name() String
    }
    
    class StorageStrategyContext {
        +storage_strategy: Arc~StorageStrategy~
        +name: String
        +bucket: Option~Bucket~
        +file_count: usize
        +node_count: usize
        +scan_tree() Result
        +set_bucket(bucket)
    }
    
    class LocalStrategy {
        +config: LocalConfig
        +scan_tree(context) Result
        +name() String
        -scan_directory(path, context) Result
        -process_file(path, context) Result
    }
    
    class SmugMugStrategy {
        +config: SmugMugConfig
        +scan_tree(context) Result
        +name() String
        -fetch_albums() Result
        -fetch_images(album) Result
    }
    
    class S3Strategy {
        +config: S3Config
        +scan_tree(context) Result
        +name() String
        -list_objects(prefix) Result
    }
    
    StorageStrategy <|.. LocalStrategy
    StorageStrategy <|.. SmugMugStrategy
    StorageStrategy <|.. S3Strategy
    StorageStrategyContext --> StorageStrategy
```

## 3. File Scanning Workflow

```mermaid
flowchart TD
    START([Start Scan]) --> LOAD[Load Configuration]
    LOAD --> INIT[Initialize Storage Strategies]
    INIT --> DB_CONN[Connect to Database]
    
    DB_CONN --> LOOP{For Each Strategy}
    LOOP --> CREATE[Create Strategy Context]
    CREATE --> BUCKET[Create/Open DB Bucket]
    BUCKET --> SCAN[Start Scanning]
    
    SCAN --> WALK[Walk Directory Tree]
    WALK --> PAR{Parallel Processing}
    
    PAR --> FILE1[Process File 1]
    PAR --> FILE2[Process File 2]
    PAR --> FILE3[Process File N]
    
    FILE1 --> META1[Extract Metadata]
    FILE2 --> META2[Extract Metadata]
    FILE3 --> META3[Extract Metadata]
    
    META1 --> HASH1[Calculate Blake3 Hash]
    META2 --> HASH2[Calculate Blake3 Hash]
    META3 --> HASH3[Calculate Blake3 Hash]
    
    HASH1 --> STORE1[Store in Database]
    HASH2 --> STORE2[Store in Database]
    HASH3 --> STORE3[Store in Database]
    
    STORE1 --> COLLECT
    STORE2 --> COLLECT
    STORE3 --> COLLECT[Collect Results]
    
    COLLECT --> NEXT{More Strategies?}
    NEXT -->|Yes| LOOP
    NEXT -->|No| REPORT[Generate Report]
    REPORT --> END([End])
```

## 4. Deduplication Process

```mermaid
flowchart LR
    subgraph "Phase 1: Analysis"
        LOAD_DB[Load Database] --> GROUP[Group by Hash]
        GROUP --> IDENTIFY[Identify Duplicates]
        IDENTIFY --> VERIFY[Verify Duplicates<br/>Optional]
    end
    
    subgraph "Phase 2: Decision"
        VERIFY --> STRATEGY{Dedup Strategy}
        STRATEGY -->|Keep Newest| NEWEST[Select Newest]
        STRATEGY -->|Keep Oldest| OLDEST[Select Oldest]
        STRATEGY -->|Keep Largest| LARGEST[Select Largest]
        STRATEGY -->|Manual| MANUAL[User Selection]
    end
    
    subgraph "Phase 3: Action"
        NEWEST --> ACTION{Action Type}
        OLDEST --> ACTION
        LARGEST --> ACTION
        MANUAL --> ACTION
        
        ACTION -->|Delete| DELETE[Remove Files]
        ACTION -->|Archive| ARCHIVE[Move to Archive]
        ACTION -->|Link| HARDLINK[Create Hard Links]
        ACTION -->|Report| REPORT[Generate Report Only]
    end
    
    subgraph "Phase 4: Cleanup"
        DELETE --> UPDATE[Update Database]
        ARCHIVE --> UPDATE
        HARDLINK --> UPDATE
        REPORT --> UPDATE
        UPDATE --> AUDIT[Create Audit Log]
    end
```

## 5. Database Schema

```mermaid
erDiagram
    BUCKET {
        string name PK
        timestamp created
        int file_count
    }
    
    FILE_ENTRY {
        string path PK
        string bucket FK
        bytes hash
        int size
        timestamp modified
        string content_type
        json metadata
    }
    
    DUPLICATE_GROUP {
        bytes hash PK
        int count
        int total_size
        timestamp first_seen
    }
    
    SCAN_HISTORY {
        uuid scan_id PK
        string bucket FK
        timestamp start_time
        timestamp end_time
        int files_scanned
        int new_files
        int updated_files
    }
    
    BUCKET ||--o{ FILE_ENTRY : contains
    FILE_ENTRY }o--|| DUPLICATE_GROUP : belongs_to
    BUCKET ||--o{ SCAN_HISTORY : has
```

## 6. Component Interaction Sequence

```mermaid
sequenceDiagram
    participant User
    participant CLI
    participant Scanner
    participant Strategy
    participant Database
    participant FileSystem
    
    User->>CLI: wupdedup-rs scan --local /path
    CLI->>Scanner: Initialize scan
    Scanner->>Strategy: Load LocalStrategy
    Scanner->>Database: Open connection
    
    loop For each file
        Strategy->>FileSystem: Read directory
        FileSystem-->>Strategy: File list
        Strategy->>FileSystem: Read file metadata
        FileSystem-->>Strategy: Metadata
        Strategy->>FileSystem: Read file content
        FileSystem-->>Strategy: Content stream
        Strategy->>Strategy: Calculate Blake3 hash
        Strategy->>Database: Store file entry
        Database-->>Strategy: Confirmation
    end
    
    Strategy-->>Scanner: Scan complete
    Scanner->>Database: Commit transaction
    Scanner-->>CLI: Statistics
    CLI-->>User: Display results
```

## 7. Data Flow Architecture

```mermaid
graph LR
    subgraph "Input Sources"
        FS[File System]
        API[Cloud APIs]
        USER[User Input]
    end
    
    subgraph "Processing Pipeline"
        READ[Read/Stream]
        PARSE[Parse Metadata]
        HASH[Hash Content]
        ANALYZE[Analyze Type]
        TRANSFORM[Transform Data]
    end
    
    subgraph "Storage"
        MEMORY[Memory Cache]
        DB[(Database)]
        INDEX[Search Index]
    end
    
    subgraph "Output"
        REPORT[Reports]
        EXPORT[Data Export]
        ACTIONS[File Actions]
    end
    
    FS --> READ
    API --> READ
    USER --> READ
    
    READ --> PARSE
    READ --> HASH
    PARSE --> ANALYZE
    HASH --> TRANSFORM
    ANALYZE --> TRANSFORM
    
    TRANSFORM --> MEMORY
    TRANSFORM --> DB
    TRANSFORM --> INDEX
    
    DB --> REPORT
    DB --> EXPORT
    DB --> ACTIONS
```

## 8. Parallel Processing Architecture

```mermaid
graph TD
    subgraph "Main Thread"
        MAIN[Main Process]
        COORD[Coordinator]
    end
    
    subgraph "Thread Pool (Rayon)"
        W1[Worker 1]
        W2[Worker 2]
        W3[Worker 3]
        W4[Worker N]
    end
    
    subgraph "Async Runtime (Tokio)"
        ASYNC1[Async Task 1]
        ASYNC2[Async Task 2]
        ASYNC3[Async Task N]
    end
    
    subgraph "Work Queue"
        QUEUE[(Channel)]
    end
    
    MAIN --> COORD
    COORD --> QUEUE
    
    QUEUE --> W1
    QUEUE --> W2
    QUEUE --> W3
    QUEUE --> W4
    
    W1 --> ASYNC1
    W2 --> ASYNC2
    W3 --> ASYNC3
    
    ASYNC1 --> DB[(Database)]
    ASYNC2 --> DB
    ASYNC3 --> DB
```

## 9. Configuration Hierarchy

```mermaid
graph BT
    subgraph "Configuration Sources"
        DEFAULT[Default Values]
        FILE1[config.toml]
        FILE2[config.local.toml]
        ENV[Environment Variables]
        CLI[CLI Arguments]
    end
    
    subgraph "Configuration Merge"
        MERGE[Config Builder]
    end
    
    subgraph "Final Configuration"
        CONFIG[Runtime Config]
    end
    
    DEFAULT --> MERGE
    FILE1 --> MERGE
    FILE2 --> MERGE
    ENV --> MERGE
    CLI --> MERGE
    
    MERGE --> CONFIG
    
    style DEFAULT fill:#f9f,stroke:#333,stroke-width:1px
    style FILE1 fill:#9ff,stroke:#333,stroke-width:1px
    style FILE2 fill:#9ff,stroke:#333,stroke-width:1px
    style ENV fill:#ff9,stroke:#333,stroke-width:1px
    style CLI fill:#9f9,stroke:#333,stroke-width:1px
```

## 10. Error Handling Flow

```mermaid
flowchart TD
    OP[Operation] --> TRY{Try Operation}
    TRY -->|Success| OK[Return Result]
    TRY -->|Error| ERR[Capture Error]
    
    ERR --> CTX[Add Context]
    CTX --> LOG[Log Error]
    LOG --> RECOVER{Recoverable?}
    
    RECOVER -->|Yes| RETRY{Retry?}
    RECOVER -->|No| PROP[Propagate Error]
    
    RETRY -->|Yes| BACKOFF[Exponential Backoff]
    RETRY -->|No| PROP
    
    BACKOFF --> TRY
    
    PROP --> HANDLE{Handler Level}
    HANDLE -->|Component| COMP[Component Handler]
    HANDLE -->|Application| APP[App Handler]
    HANDLE -->|User| USER[User Message]
    
    COMP --> FALLBACK[Fallback Action]
    APP --> CLEANUP[Cleanup Resources]
    USER --> EXIT[Graceful Exit]
```

## 11. Performance Monitoring

```mermaid
graph LR
    subgraph "Metrics Collection"
        APP[Application] --> METRICS[Metrics Collector]
        METRICS --> COUNTER[Counters]
        METRICS --> GAUGE[Gauges]
        METRICS --> HIST[Histograms]
    end
    
    subgraph "Export"
        COUNTER --> PROM[Prometheus]
        GAUGE --> PROM
        HIST --> PROM
        PROM --> GRAFANA[Grafana]
    end
    
    subgraph "Profiling"
        APP --> PROF[Profiler]
        PROF --> CPU[CPU Profile]
        PROF --> MEM[Memory Profile]
        PROF --> TRACE[Trace Profile]
    end
    
    subgraph "Analysis"
        CPU --> FLAME[Flamegraph]
        MEM --> HEAP[Heap Analysis]
        TRACE --> TIMELINE[Timeline View]
    end
```

## 12. Future Architecture: Distributed System

```mermaid
graph TB
    subgraph "Control Plane"
        SCHEDULER[Job Scheduler]
        COORD[Coordinator]
        META[(Metadata Store)]
    end
    
    subgraph "Worker Nodes"
        W1[Worker 1<br/>Local Scanner]
        W2[Worker 2<br/>Cloud Scanner]
        W3[Worker 3<br/>Dedup Engine]
        W4[Worker N<br/>Analytics]
    end
    
    subgraph "Message Queue"
        QUEUE[Task Queue]
        RESULTS[Results Queue]
    end
    
    subgraph "Storage Layer"
        DFS[Distributed FS]
        CACHE[Distributed Cache]
        DB[(Distributed DB)]
    end
    
    SCHEDULER --> QUEUE
    COORD --> META
    
    QUEUE --> W1
    QUEUE --> W2
    QUEUE --> W3
    QUEUE --> W4
    
    W1 --> RESULTS
    W2 --> RESULTS
    W3 --> RESULTS
    W4 --> RESULTS
    
    RESULTS --> COORD
    
    W1 --> DFS
    W2 --> DFS
    W3 --> DB
    W4 --> CACHE
```

## Diagram Usage Guide

These diagrams illustrate:

1. **System Architecture**: Overall component structure and relationships
2. **Storage Strategy Pattern**: Object-oriented design for extensibility
3. **File Scanning Workflow**: Step-by-step scanning process
4. **Deduplication Process**: How duplicates are identified and handled
5. **Database Schema**: Data model and relationships
6. **Component Interaction**: Sequence of operations during scanning
7. **Data Flow**: How data moves through the system
8. **Parallel Processing**: Concurrent execution model
9. **Configuration Hierarchy**: How settings are loaded and merged
10. **Error Handling**: Error propagation and recovery
11. **Performance Monitoring**: Metrics and profiling architecture
12. **Future Architecture**: Distributed system design

Each diagram can be rendered using Mermaid-compatible tools or viewers for better visualization.