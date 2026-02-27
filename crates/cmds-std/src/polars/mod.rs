pub mod types;

// Phase 1: IO
pub mod polars_read_csv;
pub mod polars_write_csv;
pub mod polars_read_json;
pub mod polars_write_json;
pub mod polars_read_parquet;
pub mod polars_write_parquet;

// Phase 1: Create
pub mod polars_create_dataframe;
pub mod polars_create_series;
pub mod polars_create_empty;
pub mod polars_from_rows;

// Phase 1: Info
pub mod polars_shape;
pub mod polars_schema;
pub mod polars_describe;
pub mod polars_head;

// Phase 1: Select
pub mod polars_select;
pub mod polars_drop;
pub mod polars_rename;
pub mod polars_get_column;

// Phase 2: Filter
pub mod polars_filter;
pub mod polars_filter_expr;
pub mod polars_unique;
pub mod polars_drop_nulls;
pub mod polars_slice;
pub mod polars_sample;

// Phase 2: Transform
pub mod polars_sort;
pub mod polars_reverse;
pub mod polars_with_column;
pub mod polars_cast;
pub mod polars_fill_null;
pub mod polars_fill_nan;
pub mod polars_with_row_index;
pub mod polars_replace;

// Phase 2: Join
pub mod polars_inner_join;
pub mod polars_left_join;
pub mod polars_full_join;
pub mod polars_cross_join;
pub mod polars_semi_join;
pub mod polars_anti_join;

// Phase 3: Aggregate
pub mod polars_sum;
pub mod polars_mean;
pub mod polars_median;
pub mod polars_min;
pub mod polars_max;
pub mod polars_std;
pub mod polars_var;
pub mod polars_quantile;
pub mod polars_count;
pub mod polars_skew;
pub mod polars_kurtosis;
pub mod polars_group_by;

// Phase 3: Reshape
pub mod polars_pivot;
pub mod polars_unpivot;
pub mod polars_explode;
pub mod polars_transpose;
pub mod polars_vstack;
pub mod polars_hstack;

// Phase 4: Series
pub mod polars_series_add;
pub mod polars_series_sub;
pub mod polars_series_mul;
pub mod polars_series_div;
pub mod polars_series_compare;
pub mod polars_series_cast;
pub mod polars_series_unique;
pub mod polars_series_sort;
pub mod polars_series_len;
pub mod polars_series_sum;
pub mod polars_series_mean;
pub mod polars_series_min_max;
pub mod polars_str_operations;

// Phase 4: Window
pub mod polars_shift;
pub mod polars_cumsum;
pub mod polars_cumprod;
pub mod polars_cummin;
pub mod polars_cummax;
pub mod polars_rolling_mean;
pub mod polars_rolling_sum;
pub mod polars_rank;
