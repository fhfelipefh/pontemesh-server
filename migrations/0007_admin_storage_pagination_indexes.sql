CREATE INDEX IF NOT EXISTS idx_buckets_created_at ON buckets(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_buckets_name_lower ON buckets(lower(name));
CREATE INDEX IF NOT EXISTS idx_objects_bucket_updated ON objects(bucket_id, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_objects_bucket_key_lower ON objects(bucket_id, lower(object_key));
CREATE INDEX IF NOT EXISTS idx_object_versions_created_at ON object_versions(created_at DESC);
