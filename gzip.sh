for f in test_files/*; do
    if [[ ! $f == *.gz ]]; then
        if [ ! -f "$f.gz" ]; then
            echo "gzip -k $f"
            gzip -k $f
        else
            echo "exist $f"
        fi
    fi
done
