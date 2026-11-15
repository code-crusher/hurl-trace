Set-StrictMode -Version latest
$ErrorActionPreference = 'Stop'

hurl --parallel --jobs 1 --repeat 2 --job-variable NAME=red,green,blue `
  tests_ok/parallel/parallel_job_variable.hurl `
  tests_ok/parallel/parallel_job_variable.hurl `
  tests_ok/parallel/parallel_job_variable.hurl
