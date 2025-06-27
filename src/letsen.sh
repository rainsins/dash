#!/bin/bash

# SSL证书自动更新和部署脚本
# 用途：更新Let's Encrypt证书并部署到指定目录

set -e  # 遇到错误立即退出

# 配置变量
DOMAIN="rainsin.cn"                           # 您的域名
EMAIL="1820278582@qq.com"                # 用于Let's Encrypt注册的邮箱
WEBROOT_PATH="/var/www/html"                  # 网站根目录（webroot模式）
SOURCE_DIR="/etc/letsencrypt/live/$DOMAIN"    # 证书源目录
TARGET_DIR="/home/ubuntu/.ssl/$DOMAIN"        # 目标目录
SERVICE_GROUP="ubuntu"                        # 使用证书的服务组名
NATFRP_PATH="/etc/natfrp/FrpcWorkingDirectory" # NATFRP目录
LOG_FILE="/var/log/certbot-deploy.log"        # 日志文件

# 证书更新配置
CERT_RENEW_DAYS=30                           # 证书到期前多少天开始续期
FORCE_RENEW=false                            # 是否强制续期
USE_STANDALONE=false                         # 是否使用standalone模式（需要停止web服务）

# 需要重启的服务列表
SERVICES_TO_RESTART=("caddy")      # 根据实际情况修改

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 日志函数
log() {
    local message="$1"
    echo -e "${GREEN}[$(date '+%Y-%m-%d %H:%M:%S')]${NC} $message"
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $message" >> "$LOG_FILE"
}

error() {
    local message="$1"
    echo -e "${RED}[ERROR]${NC} $message" >&2
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] ERROR: $message" >> "$LOG_FILE"
    exit 1
}

warning() {
    local message="$1"
    echo -e "${YELLOW}[WARNING]${NC} $message"
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] WARNING: $message" >> "$LOG_FILE"
}

info() {
    local message="$1"
    echo -e "${BLUE}[INFO]${NC} $message"
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] INFO: $message" >> "$LOG_FILE"
}

# 检查是否以root权限运行
check_root() {
    if [[ $EUID -ne 0 ]]; then
       error "此脚本需要root权限运行，请使用 sudo"
    fi
}

# 检查必要的命令是否存在
check_dependencies() {
    local deps=("certbot" "openssl")
    
    for cmd in "${deps[@]}"; do
        if ! command -v "$cmd" &> /dev/null; then
            error "必需的命令 '$cmd' 未安装，请先安装"
        fi
    done
    
    log "依赖检查通过"
}

# 安装certbot（如果未安装）
install_certbot() {
    if ! command -v certbot &> /dev/null; then
        log "检测到certbot未安装，开始安装..."
        
        # 检测系统类型并安装
        if command -v apt-get &> /dev/null; then
            apt-get update
            apt-get install -y certbot python3-certbot-nginx
        elif command -v yum &> /dev/null; then
            yum install -y epel-release
            yum install -y certbot python3-certbot-nginx
        elif command -v dnf &> /dev/null; then
            dnf install -y certbot python3-certbot-nginx
        else
            error "无法自动安装certbot，请手动安装"
        fi
        
        log "certbot安装完成"
    fi
}

# 检查证书状态
check_certificate_status() {
    local cert_file="$SOURCE_DIR/cert.pem"
    
    if [[ ! -f "$cert_file" ]]; then
        info "证书文件不存在，需要首次申请证书"
        return 1
    fi
    
    # 检查证书有效期
    local expiry_date=$(openssl x509 -in "$cert_file" -noout -enddate | cut -d= -f2)
    local expiry_timestamp=$(date -d "$expiry_date" +%s)
    local current_timestamp=$(date +%s)
    local days_until_expiry=$(( (expiry_timestamp - current_timestamp) / 86400 ))
    
    info "证书到期时间: $expiry_date"
    info "距离到期还有: $days_until_expiry 天"
    
    if [[ $days_until_expiry -le $CERT_RENEW_DAYS ]]; then
        info "证书即将到期，需要续期"
        return 1
    else
        info "证书仍在有效期内"
        return 0
    fi
}

# 申请新证书
obtain_certificate() {
    log "开始申请Let's Encrypt证书..."
    
    local certbot_cmd="certbot certonly"
    local certbot_args="--non-interactive --agree-tos --email $EMAIL -d $DOMAIN"
    
    if [[ "$USE_STANDALONE" == "true" ]]; then
        info "使用standalone模式申请证书"
        # 停止web服务以释放80端口
        stop_web_services
        certbot_cmd="$certbot_cmd --standalone $certbot_args"
    else
        info "使用webroot模式申请证书"
        # 确保webroot目录存在
        create_directory "$WEBROOT_PATH" "网站根"
        certbot_cmd="$certbot_cmd --webroot -w $WEBROOT_PATH $certbot_args"
    fi
    
    # 执行证书申请
    if eval "$certbot_cmd"; then
        log "证书申请成功"
        if [[ "$USE_STANDALONE" == "true" ]]; then
            start_web_services
        fi
    else
        error "证书申请失败"
    fi
}

# 续期证书
renew_certificate() {
    log "开始续期Let's Encrypt证书..."
    
    local renew_cmd="certbot renew --quiet"
    
    if [[ "$FORCE_RENEW" == "true" ]]; then
        renew_cmd="$renew_cmd --force-renewal"
        info "强制续期模式"
    fi
    
    if [[ "$USE_STANDALONE" == "true" ]]; then
        stop_web_services
        renew_cmd="$renew_cmd --standalone"
    fi
    
    # 添加续期前的验证测试
    log "执行续期前测试..."
    if ! certbot renew --dry-run --quiet; then
        warning "续期测试失败，可能存在配置问题"
        if [[ "$USE_STANDALONE" == "true" ]]; then
            start_web_services
        fi
        return 1
    fi
    
    # 执行证书续期
    log "执行实际续期..."
    if eval "$renew_cmd"; then
        log "证书续期成功"
        
        # 验证新证书
        if verify_certificate; then
            log "新证书验证通过"
        else
            warning "新证书验证失败"
        fi
        
        if [[ "$USE_STANDALONE" == "true" ]]; then
            start_web_services
        fi
        return 0
    else
        error_msg=$(certbot renew 2>&1 | tail -10)
        warning "证书续期失败，错误信息: $error_msg"
        
        # 发送通知邮件（如果配置了）
        send_notification "证书续期失败" "Domain: $DOMAIN\nError: $error_msg"
        
        if [[ "$USE_STANDALONE" == "true" ]]; then
            start_web_services
        fi
        return 1
    fi
}

# 停止web服务
stop_web_services() {
    for service in "${SERVICES_TO_RESTART[@]}"; do
        if systemctl is-active --quiet "$service"; then
            log "停止服务: $service"
            systemctl stop "$service" || warning "停止服务失败: $service"
        fi
    done
}

# 启动web服务
start_web_services() {
    for service in "${SERVICES_TO_RESTART[@]}"; do
        if systemctl is-enabled --quiet "$service" 2>/dev/null; then
            log "启动服务: $service"
            systemctl start "$service" || warning "启动服务失败: $service"
        fi
    done
}

# 重启相关服务
restart_services() {
    log "重启相关服务..."
    
    for service in "${SERVICES_TO_RESTART[@]}"; do
        if systemctl is-active --quiet "$service"; then
            log "重启服务: $service"
            if systemctl restart "$service"; then
                log "服务 $service 重启成功"
            else
                warning "服务 $service 重启失败"
            fi
        else
            info "服务 $service 未运行，跳过重启"
        fi
    done
}

# 创建目录函数
create_directory() {
    local dir="$1"
    local description="$2"
    
    if [[ ! -d "$dir" ]]; then
        log "创建$description目录: $dir"
        if ! mkdir -p "$dir"; then
            error "无法创建目录: $dir"
        fi
    else
        log "$description目录已存在: $dir"
    fi
}

# 复制文件函数
copy_file() {
    local source="$1"
    local target="$2"
    local description="$3"
    
    log "复制$description: $source -> $target"
    if ! cp -L "$source" "$target"; then
        error "复制文件失败: $source -> $target"
    fi
}

# 设置文件权限函数
set_permissions() {
    local file="$1"
    local permissions="$2"
    local description="$3"
    
    if [[ -f "$file" ]]; then
        log "设置$description权限: $file ($permissions)"
        if ! chmod "$permissions" "$file"; then
            warning "设置权限失败: $file"
        fi
    else
        warning "文件不存在，跳过权限设置: $file"
    fi
}

# 设置文件所有权函数
set_ownership() {
    local path="$1"
    local owner="$2"
    local description="$3"
    
    if [[ -e "$path" ]]; then
        log "设置$description所有权: $path ($owner)"
        if ! chown "$owner" "$path"; then
            warning "设置所有权失败: $path"
        fi
    else
        warning "路径不存在，跳过所有权设置: $path"
    fi
}

# 部署证书文件
deploy_certificates() {
    log "开始部署证书文件..."
    
    # 检查源证书文件是否存在
    for file in "privkey.pem" "fullchain.pem" "cert.pem"; do
        if [[ ! -f "$SOURCE_DIR/$file" ]]; then
            error "证书文件不存在: $SOURCE_DIR/$file"
        fi
    done
    
    # 创建必要的目录
    create_directory "$TARGET_DIR" "目标SSL证书"
    create_directory "$(dirname "$NATFRP_PATH")" "NATFRP父"
    create_directory "$NATFRP_PATH" "NATFRP工作"
    
    # 复制证书文件到目标目录（多种格式）
    copy_file "$SOURCE_DIR/privkey.pem" "$TARGET_DIR/private.key" "私钥(.key)"
    copy_file "$SOURCE_DIR/fullchain.pem" "$TARGET_DIR/fullchain.crt" "完整证书链(.crt)"
    copy_file "$SOURCE_DIR/cert.pem" "$TARGET_DIR/certificate.crt" "证书(.crt)"
    
    copy_file "$SOURCE_DIR/privkey.pem" "$TARGET_DIR/private.pem" "私钥(.pem)"
    copy_file "$SOURCE_DIR/fullchain.pem" "$TARGET_DIR/fullchain.pem" "完整证书链(.pem)"
    copy_file "$SOURCE_DIR/cert.pem" "$TARGET_DIR/certificate.pem" "证书(.pem)"
    
    # 复制证书文件到NATFRP目录
    copy_file "$SOURCE_DIR/privkey.pem" "$NATFRP_PATH/$DOMAIN.key" "NATFRP私钥"
    copy_file "$SOURCE_DIR/fullchain.pem" "$NATFRP_PATH/$DOMAIN.crt" "NATFRP证书"
    
    # 设置文件权限
    set_permissions "$TARGET_DIR/private.key" "640" "私钥"
    set_permissions "$TARGET_DIR/fullchain.crt" "644" "证书链"
    set_permissions "$TARGET_DIR/certificate.crt" "644" "证书"
    set_permissions "$TARGET_DIR/private.pem" "640" "私钥(PEM)"
    set_permissions "$TARGET_DIR/fullchain.pem" "644" "证书链(PEM)"
    set_permissions "$TARGET_DIR/certificate.pem" "644" "证书(PEM)"
    
    # 设置NATFRP文件权限
    set_permissions "$NATFRP_PATH/$DOMAIN.key" "640" "NATFRP私钥"
    set_permissions "$NATFRP_PATH/$DOMAIN.crt" "644" "NATFRP证书"
    
    # 设置文件所有权
    for file in "$TARGET_DIR"/*; do
        if [[ -f "$file" ]]; then
            set_ownership "$file" "root:$SERVICE_GROUP" "目标目录文件"
        fi
    done
    
    set_ownership "$NATFRP_PATH/$DOMAIN.key" "root:$SERVICE_GROUP" "NATFRP私钥"
    set_ownership "$NATFRP_PATH/$DOMAIN.crt" "root:$SERVICE_GROUP" "NATFRP证书"
    
    log "证书部署完成"
}

# 验证证书有效性
verify_certificate() {
    local cert_file="$SOURCE_DIR/cert.pem"
    
    if [[ ! -f "$cert_file" ]]; then
        warning "证书文件不存在: $cert_file"
        return 1
    fi
    
    # 检查证书是否有效
    if ! openssl x509 -in "$cert_file" -noout -checkend 86400; then
        warning "证书将在24小时内过期或已过期"
        return 1
    fi
    
    # 检查证书域名
    local cert_domains=$(openssl x509 -in "$cert_file" -noout -text | grep -A1 "Subject Alternative Name" | tail -1 | sed 's/DNS://g' | tr ',' '\n' | tr -d ' ')
    if ! echo "$cert_domains" | grep -q "^$DOMAIN$"; then
        warning "证书不包含域名: $DOMAIN"
        return 1
    fi
    
    log "证书验证通过"
    return 0
}

# 发送通知邮件（可选功能）
send_notification() {
    local subject="$1"
    local message="$2"
    local notify_email="${NOTIFY_EMAIL:-$EMAIL}"
    
    if [[ -n "$notify_email" ]] && command -v mail &> /dev/null; then
        echo -e "$message" | mail -s "[$DOMAIN] $subject" "$notify_email"
        log "通知邮件已发送到: $notify_email"
    fi

    if [[ -f "$SOURCE_DIR/cert.pem" ]] && command -v openssl &> /dev/null; then
        log "证书信息:"
        echo "----------------------------------------"
        openssl x509 -in "$SOURCE_DIR/cert.pem" -noout -subject -issuer -dates 2>/dev/null || warning "无法读取证书信息"
        echo "----------------------------------------"
    fi
}

# 命令行参数处理
parse_arguments() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --force-renew)
                FORCE_RENEW=true
                info "启用强制续期模式"
                shift
                ;;
            --standalone)
                USE_STANDALONE=true
                info "使用standalone模式"
                shift
                ;;
            --email)
                EMAIL="$2"
                info "设置邮箱: $EMAIL"
                shift 2
                ;;
            --domain)
                DOMAIN="$2"
                SOURCE_DIR="/etc/letsencrypt/live/$DOMAIN"
                TARGET_DIR="/home/ubuntu/.ssl/$DOMAIN"
                info "设置域名: $DOMAIN"
                shift 2
                ;;
            --webroot)
                WEBROOT_PATH="$2"
                info "设置webroot路径: $WEBROOT_PATH"
                shift 2
                ;;
            --help|-h)
                show_help
                exit 0
                ;;
            *)
                warning "未知参数: $1"
                shift
                ;;
        esac
    done
}

# 显示帮助信息
show_help() {
    cat << EOF
SSL证书自动更新和部署脚本

用法: $0 [选项]

选项:
    --force-renew       强制续期证书
    --standalone        使用standalone模式（需要停止web服务）
    --email EMAIL       设置Let's Encrypt注册邮箱
    --domain DOMAIN     设置域名
    --webroot PATH      设置webroot路径
    --help, -h          显示此帮助信息

示例:
    $0                                          # 使用默认配置
    $0 --force-renew                           # 强制续期
    $0 --standalone --domain example.com       # 使用standalone模式
    $0 --email admin@example.com               # 指定邮箱

EOF
}

# 主要执行流程
main() {
    log "开始SSL证书自动更新和部署脚本"
    log "域名: $DOMAIN"
    log "邮箱: $EMAIL"
    
    # 解析命令行参数
    parse_arguments "$@"
    
    # 检查运行环境
    check_root
    check_dependencies
    
    # 创建日志目录和文件
    create_directory "$(dirname "$LOG_FILE")" "日志"
    if [[ ! -f "$LOG_FILE" ]]; then
        touch "$LOG_FILE"
        chmod 644 "$LOG_FILE"
    fi
    
    # 安装certbot（如果需要）
    install_certbot
    
    # 检查证书状态并决定是否需要更新
    local need_certificate=false
    
    if ! check_certificate_status || [[ "$FORCE_RENEW" == "true" ]]; then
        need_certificate=true
    fi
    
    # 更新或申请证书
    if [[ "$need_certificate" == "true" ]]; then
        if [[ -d "$SOURCE_DIR" ]]; then
            # 证书目录存在，尝试续期
            if renew_certificate; then
                log "证书续期成功，开始部署"
            else
                info "续期失败，尝试重新申请证书"
                obtain_certificate
            fi
        else
            # 证书目录不存在，首次申请
            obtain_certificate
        fi
        
        # 部署证书
        deploy_certificates
        
        # 重启相关服务
        restart_services
        
        log "证书更新和部署完成！"
    else
        log "证书仍在有效期内，仅执行部署"
        deploy_certificates
    fi
    
    # 显示证书信息
    # show_certificate_info
    
    log "脚本执行完成"
    log "证书已部署到: $TARGET_DIR"
    log "NATFRP证书已部署到: $NATFRP_PATH"
}

# 执行主函数
main "$@"